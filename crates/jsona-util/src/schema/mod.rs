use anyhow::{anyhow, bail};
use jsona::dom::{Keys, Node};
use parking_lot::Mutex;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use url::Url;

use self::{associations::SchemaAssociations, cache::Cache};
use crate::{environment::Environment, LruCache};

pub use jsona_schema_validator::{
    JSONASchemaValidator, JSONASchemaValue, NodeValidationError, Schema,
};

pub mod associations;
pub mod cache;

#[derive(Clone)]
pub struct Schemas<E: Environment> {
    env: E,
    associations: SchemaAssociations<E>,
    concurrent_requests: Arc<Semaphore>,
    validators: Arc<Mutex<LruCache<Url, Arc<JSONASchemaValidator>>>>,
    cache_schema: Cache<E, JSONASchemaValue>,
    cache_value: Cache<E, Value>,
}

impl<E: Environment> Schemas<E> {
    pub fn new(env: E) -> Self {
        let cache_schema = Cache::new(env.clone());
        let cache_value = Cache::new(env.clone());

        Self {
            associations: SchemaAssociations::new(env.clone(), cache_value.clone()),
            cache_schema,
            cache_value,
            env,
            concurrent_requests: Arc::new(Semaphore::new(10)),
            validators: Arc::new(Mutex::new(LruCache::new(3))),
        }
    }

    /// Get a reference to the schemas's associations.
    pub fn associations(&self) -> &SchemaAssociations<E> {
        &self.associations
    }

    pub fn set_cache_path(&self, path: Option<PathBuf>) {
        tracing::debug!("set cache path {:?}", path);
        self.cache_schema.set_cache_path(path.clone());
        self.cache_value.set_cache_path(path);
    }

    pub fn set_cache_expiration_times(&self, mem: Duration, disk: Duration) {
        tracing::debug!("set cache expiration time, mem: {:?} disk: {:?}", mem, disk);
        self.cache_schema.set_expiration_times(mem, disk);
        self.cache_schema.set_expiration_times(mem, disk);
    }

    pub fn env(&self) -> &E {
        &self.env
    }
}

impl<E: Environment> Schemas<E> {
    #[tracing::instrument(skip_all, fields(%schema_url))]
    pub async fn validate(
        &self,
        schema_url: &Url,
        value: &Node,
    ) -> Result<Vec<NodeValidationError>, anyhow::Error> {
        let validator = match self.get_validator(schema_url) {
            Some(s) => s,
            None => {
                let schema = self
                    .load_schema(schema_url)
                    .await
                    .map_err(|err| anyhow!("failed to load schema {schema_url} {}", err))?;
                self.add_schema(schema_url, schema.clone()).await;
                self.add_validator(schema_url.clone(), &schema)
                    .map_err(|err| anyhow!("load schema {schema_url} throw {}", err))?
            }
        };
        Ok(validator.validate(value))
    }

    pub async fn add_schema(&self, schema_url: &Url, schema: Arc<JSONASchemaValue>) {
        drop(self.cache_schema.store(schema_url.clone(), schema).await);
    }

    #[tracing::instrument(skip_all, fields(%schema_url))]
    pub async fn load_schema(
        &self,
        schema_url: &Url,
    ) -> Result<Arc<JSONASchemaValue>, anyhow::Error> {
        if let Ok(s) = self.cache_schema.load(schema_url, false).await {
            tracing::debug!(%schema_url, "schema was found in cache");
            return Ok(s);
        }
        let schema: Arc<JSONASchemaValue> =
            match self.fetch_external(schema_url).await.and_then(|v| {
                std::str::from_utf8(&v)
                    .map_err(|v| anyhow!("{}", v))
                    .and_then(|v| v.parse().map_err(|err| anyhow!("{}", err)))
            }) {
                Ok(s) => Arc::new(s),
                Err(error) => {
                    tracing::warn!(?error, "failed to fetch remote schema");
                    if let Ok(s) = self.cache_schema.load(schema_url, true).await {
                        tracing::debug!(%schema_url, "expired schema was found in cache");
                        return Ok(s);
                    }
                    return Err(error);
                }
            };

        if let Err(error) = self
            .cache_schema
            .store(schema_url.clone(), schema.clone())
            .await
        {
            tracing::debug!(%error, "failed to cache schema");
        }

        Ok(schema)
    }

    #[tracing::instrument(skip_all, fields(%schema_url, %path))]
    pub async fn schemas_at_path(
        &self,
        schema_url: &Url,
        path: &Keys,
    ) -> Result<Vec<Schema>, anyhow::Error> {
        let schema = self.load_schema(schema_url).await?;
        let schemas = schema.pointer(path).into_iter().cloned().collect();
        Ok(schemas)
    }

    fn get_validator(&self, schema_url: &Url) -> Option<Arc<JSONASchemaValidator>> {
        if self.cache_schema.lru_expired() {
            self.validators.lock().clear();
        }

        self.validators.lock().get(schema_url).cloned()
    }

    fn add_validator(
        &self,
        schema_url: Url,
        schema: &JSONASchemaValue,
    ) -> Result<Arc<JSONASchemaValidator>, anyhow::Error> {
        let v = Arc::new(JSONASchemaValidator::new(schema)?);
        self.validators.lock().put(schema_url, v.clone());
        Ok(v)
    }

    async fn fetch_external(&self, index_url: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let _permit = self.concurrent_requests.acquire().await?;
        let data: Vec<u8> = match index_url.scheme() {
            "http" | "https" => self.env.fetch_file(index_url).await?,
            "file" => {
                self.env
                    .read_file(
                        self.env
                            .to_file_path(index_url)
                            .ok_or_else(|| anyhow!("invalid file path"))?
                            .as_ref(),
                    )
                    .await?
            }
            scheme => bail!("the scheme `{scheme}` is not supported"),
        };
        Ok(data)
    }
}
