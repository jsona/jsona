pub mod associations;
pub mod fetcher;

use anyhow::anyhow;
use jsona::dom::{Keys, Node};
use parking_lot::Mutex;
use std::{str::FromStr, sync::Arc};
use url::Url;

use self::associations::SchemaAssociations;
use self::fetcher::Fetcher;
use crate::{environment::Environment, HashMap};

pub use jsona_schema_validator::{JSONASchemaValidationError, JSONASchemaValidator, Schema};

#[derive(Clone)]
pub struct Schemas<E: Environment> {
    associations: SchemaAssociations<E>,
    fetcher: Fetcher<E>,
    validators: Arc<Mutex<HashMap<Url, Arc<JSONASchemaValidator>>>>,
}

impl<E: Environment> Schemas<E> {
    pub fn new(env: E) -> Self {
        let fetcher = Fetcher::new(env.clone());
        Self {
            associations: SchemaAssociations::new(env, fetcher.clone()),
            fetcher,
            validators: Arc::new(Mutex::new(HashMap::default())),
        }
    }

    /// Get a reference to the schemas's associations.
    pub fn associations(&self) -> &SchemaAssociations<E> {
        &self.associations
    }

    pub fn set_cache_path(&self, path: Option<Url>) {
        tracing::info!("set cache path {:?}", path.as_ref().map(|v| v.as_str()));
        self.fetcher.set_cache_path(path);
    }
}

impl<E: Environment> Schemas<E> {
    #[tracing::instrument(skip_all, fields(%schema_uri))]
    pub async fn validate(
        &self,
        schema_uri: &Url,
        value: &Node,
    ) -> Result<Vec<JSONASchemaValidationError>, anyhow::Error> {
        let validator = self.load_validator(schema_uri).await?;
        Ok(validator.validate(value))
    }

    pub async fn load_validator(
        &self,
        schema_uri: &Url,
    ) -> Result<Arc<JSONASchemaValidator>, anyhow::Error> {
        if let Some(s) = self.validators.lock().get(schema_uri).cloned() {
            return Ok(s);
        }

        let schema: Arc<JSONASchemaValidator> =
            match self.fetcher.fetch(schema_uri).await.and_then(|v| {
                std::str::from_utf8(&v)
                    .map_err(|v| anyhow!("{}", v))
                    .and_then(|v| Node::from_str(v).map_err(|err| anyhow!("{}", err)))
                    .and_then(|v| {
                        JSONASchemaValidator::try_from(&v).map_err(|err| anyhow!("{}", err))
                    })
            }) {
                Ok(s) => Arc::new(s),
                Err(error) => {
                    tracing::warn!(?error, "failed to use remote jsonaschema");
                    return Err(error);
                }
            };

        self.validators
            .lock()
            .insert(schema_uri.clone(), schema.clone());

        Ok(schema)
    }

    #[tracing::instrument(skip_all, fields(%schema_uri))]
    pub async fn query(&self, schema_uri: &Url, path: &Keys) -> Result<Vec<Schema>, anyhow::Error> {
        let validator = self.load_validator(schema_uri).await?;
        let schemas = validator.pointer(path).into_iter().cloned().collect();
        Ok(schemas)
    }
}
