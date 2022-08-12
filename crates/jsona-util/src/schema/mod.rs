pub mod associations;
pub mod fetcher;

use anyhow::anyhow;
use jsona::dom::{Keys, Node};
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc};
use url::Url;

use self::associations::SchemaAssociations;
use self::fetcher::Fetcher;
use crate::{environment::Environment, HashMap};

pub use jsona_schema_validator::{
    JSONASchemaValidator, JSONASchemaValue, NodeValidationError, Schema,
};

#[derive(Clone)]
pub struct Schemas<E: Environment> {
    env: E,
    associations: SchemaAssociations<E>,
    fetcher: Fetcher<E>,
    validators: Arc<Mutex<HashMap<Url, Arc<JSONASchemaValidator>>>>,
    schemas: Arc<Mutex<HashMap<Url, Arc<JSONASchemaValue>>>>,
}

impl<E: Environment> Schemas<E> {
    pub fn new(env: E) -> Self {
        let fetcher = Fetcher::new(env.clone());
        Self {
            associations: SchemaAssociations::new(env.clone(), fetcher.clone()),
            fetcher,
            env,
            validators: Arc::new(Mutex::new(HashMap::default())),
            schemas: Arc::new(Mutex::new(HashMap::default())),
        }
    }

    /// Get a reference to the schemas's associations.
    pub fn associations(&self) -> &SchemaAssociations<E> {
        &self.associations
    }

    pub fn env(&self) -> &E {
        &self.env
    }

    pub fn set_cache_path(&self, path: Option<PathBuf>) {
        tracing::debug!("set cache path {:?}", path);
        self.fetcher.set_cache_path(path);
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
                self.add_schema(schema_url, schema.clone());
                self.add_validator(schema_url.clone(), &schema)
                    .map_err(|err| anyhow!("load schema {schema_url} throw {}", err))?
            }
        };
        Ok(validator.validate(value))
    }

    pub fn add_schema(&self, schema_url: &Url, schema: Arc<JSONASchemaValue>) {
        drop(self.schemas.lock().insert(schema_url.clone(), schema));
    }

    pub async fn load_schema(
        &self,
        schema_url: &Url,
    ) -> Result<Arc<JSONASchemaValue>, anyhow::Error> {
        if let Some(s) = self.schemas.lock().get(schema_url).cloned() {
            tracing::debug!(%schema_url, "schema was found in cache");
            return Ok(s);
        }

        let schema: Arc<JSONASchemaValue> =
            match self.fetcher.fetch(schema_url).await.and_then(|v| {
                std::str::from_utf8(&v)
                    .map_err(|v| anyhow!("{}", v))
                    .and_then(|v| v.parse().map_err(|err| anyhow!("{}", err)))
            }) {
                Ok(s) => Arc::new(s),
                Err(error) => {
                    tracing::warn!(?error, "failed to fetch remote schema");
                    return Err(error);
                }
            };

        self.schemas
            .lock()
            .insert(schema_url.clone(), schema.clone());

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
        self.validators.lock().get(schema_url).cloned()
    }

    fn add_validator(
        &self,
        schema_url: Url,
        schema: &JSONASchemaValue,
    ) -> Result<Arc<JSONASchemaValidator>, anyhow::Error> {
        let v = Arc::new(JSONASchemaValidator::new(schema)?);
        self.validators.lock().insert(schema_url, v.clone());
        Ok(v)
    }
}
