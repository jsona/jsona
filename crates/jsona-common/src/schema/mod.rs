use anyhow::{anyhow, bail, Context};
use jsona::{
    dom::{Keys, Node},
    parser::parse,
};
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;
use url::Url;

use self::{
    associations::SchemaAssociations,
    cache::Cache,
    jsona_schema::{JsonaSchema, ValidationError},
};
use crate::{environment::Environment, LruCache};

pub mod associations;
pub mod cache;
pub mod jsona_schema;

#[derive(Clone)]
pub struct Schemas<E: Environment> {
    env: E,
    associations: SchemaAssociations,
    concurrent_requests: Arc<Semaphore>,
    http: reqwest::Client,
    validators: Arc<Mutex<LruCache<Url, Arc<JsonaSchema>>>>,
    cache: Cache<E>,
}

impl<E: Environment> Schemas<E> {
    pub fn new(env: E, http: reqwest::Client) -> Self {
        let cache = Cache::new(env.clone());

        Self {
            associations: SchemaAssociations::default(),
            cache,
            env,
            concurrent_requests: Arc::new(Semaphore::new(10)),
            http,
            validators: Arc::new(Mutex::new(LruCache::new(3))),
        }
    }

    /// Get a reference to the schemas's associations.
    pub fn associations(&self) -> &SchemaAssociations {
        &self.associations
    }

    /// Get a reference to the schemas's cache.
    pub fn cache(&self) -> &Cache<E> {
        &self.cache
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
    ) -> Result<Vec<ValidationError>, anyhow::Error> {
        let validator = match self.get_validator(schema_url) {
            Some(s) => s,
            None => {
                let schema = self
                    .load_schema(schema_url)
                    .await
                    .with_context(|| format!("failed to load schema {schema_url}"))?;
                self.add_schema(schema_url, schema.clone()).await;
                self.add_validator(schema_url.clone(), &schema)
                    .with_context(|| format!("invalid schema {schema_url}"))?
            }
        };
        match validator.validate(value) {
            Ok(_) => Ok(vec![]),
            Err(errs) => Ok(errs),
        }
    }

    pub async fn add_schema(&self, schema_url: &Url, schema: Arc<Value>) {
        drop(self.cache.store(schema_url.clone(), schema).await);
    }

    #[tracing::instrument(skip_all, fields(%schema_url))]
    pub async fn load_schema(&self, schema_url: &Url) -> Result<Arc<Value>, anyhow::Error> {
        if let Ok(s) = self.cache.load(schema_url, false).await {
            tracing::debug!(%schema_url, "schema was found in cache");
            return Ok(s);
        }

        let schema = match self.fetch_external(schema_url).await {
            Ok(s) => Arc::new(s),
            Err(error) => {
                tracing::warn!(?error, "failed to fetch remote schema");
                if let Ok(s) = self.cache.load(schema_url, true).await {
                    tracing::debug!(%schema_url, "expired schema was found in cache");
                    return Ok(s);
                }
                return Err(error);
            }
        };

        if let Err(error) = self.cache.store(schema_url.clone(), schema.clone()).await {
            tracing::debug!(%error, "failed to cache schema");
        }

        Ok(schema)
    }

    #[tracing::instrument(skip_all, fields(%schema_url, %path))]
    pub async fn schemas_at_path(
        &self,
        schema_url: &Url,
        _node: &Node,
        path: &Keys,
    ) -> Result<Vec<(Keys, Arc<JsonaSchema>)>, anyhow::Error> {
        todo!()
    }

    #[tracing::instrument(skip_all, fields(%schema_url, %path))]
    pub async fn possible_schemas_from(
        &self,
        schema_url: &Url,
        _node: &Node,
        path: &Keys,
        _max_depth: usize,
    ) -> Result<Vec<(Keys, Keys, Arc<JsonaSchema>)>, anyhow::Error> {
        todo!()
    }

    fn get_validator(&self, schema_url: &Url) -> Option<Arc<JsonaSchema>> {
        if self.cache().lru_expired() {
            self.validators.lock().clear();
        }

        self.validators.lock().get(schema_url).cloned()
    }

    fn add_validator(
        &self,
        schema_url: Url,
        schema: &Value,
    ) -> Result<Arc<JsonaSchema>, anyhow::Error> {
        let v = Arc::new(self.create_validator(schema)?);
        self.validators.lock().put(schema_url, v.clone());
        Ok(v)
    }

    fn create_validator(&self, schema: &Value) -> Result<JsonaSchema, anyhow::Error> {
        JsonaSchema::compile(schema).map_err(|err| anyhow!("invalid schema: {err}"))
    }

    async fn fetch_external(&self, index_url: &Url) -> Result<Value, anyhow::Error> {
        let _permit = self.concurrent_requests.acquire().await?;
        let data: Vec<u8> = match index_url.scheme() {
            "http" | "https" => self
                .http
                .get(index_url.clone())
                .send()
                .await?
                .bytes()
                .await?
                .to_vec(),
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
        let data = std::str::from_utf8(&data)?;
        let node = parse(data).into_dom();
        if let Err(errors) = node.validate() {
            for error in errors {
                tracing::warn!(?error, "err was found in schema `{}`", index_url);
            }
        }
        Ok(node.to_json())
    }
}
