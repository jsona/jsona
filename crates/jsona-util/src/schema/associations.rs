use anyhow::{anyhow, bail};
use jsona::dom::{KeyOrIndex, Node};
use parking_lot::{RwLock, RwLockReadGuard};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tap::Tap;
use tokio::sync::Semaphore;
use url::Url;

use crate::{
    config::Config,
    environment::Environment,
    util::{to_file_url, GlobRule},
};

use super::cache::Cache;

pub const DEFAULT_SCHEMASTORES: &[&str] =
    &["https://cdn.jsdelivr.net/npm/@jsona/schemastore@latest/index.json"];

pub const SCHEMA_KEY: &str = "@jsonaschema";

pub mod priority {
    pub const STORE: usize = 30;
    pub const CONFIG: usize = 50;
    pub const LSP_CONFIG: usize = 60;
    pub const SCHEMA_FIELD: usize = 75;
    pub const MAX: usize = usize::MAX;
}

pub mod source {
    pub const STORE: &str = "store";
    pub const CONFIG: &str = "config";
    pub const LSP_CONFIG: &str = "lsp_config";
    pub const MANUAL: &str = "manual";
    pub const SCHEMA_FIELD: &str = "$schema";
}

#[derive(Clone)]
pub struct SchemaAssociations<E: Environment> {
    concurrent_requests: Arc<Semaphore>,
    associations: Arc<RwLock<Vec<(AssociationRule, SchemaAssociation)>>>,
    env: E,
    cache: Cache<E, Value>,
}

impl<E: Environment> SchemaAssociations<E> {
    pub(crate) fn new(env: E, cache: Cache<E, Value>) -> Self {
        Self {
            concurrent_requests: Arc::new(Semaphore::new(10)),
            cache,
            env,
            associations: Default::default(),
        }
    }
    pub fn add(&self, rule: AssociationRule, assoc: SchemaAssociation) {
        self.associations.write().push((rule, assoc));
    }

    pub fn retain(&self, f: impl Fn(&(AssociationRule, SchemaAssociation)) -> bool) {
        self.associations.write().retain(f);
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Vec<(AssociationRule, SchemaAssociation)>> {
        self.associations.read()
    }

    pub fn clear(&self) {
        self.associations.write().clear();
    }

    pub async fn add_from_schemastore(&self, url: &Url) -> Result<(), anyhow::Error> {
        let schemastore = self.load_schemastore(url).await?;
        for schema in &schemastore.0 {
            match GlobRule::new(&schema.file_match, [] as [&str; 0]) {
                Ok(rule) => {
                    self.associations.write().push((
                        rule.into(),
                        SchemaAssociation {
                            url: schema.url.clone(),
                            meta: json!({
                                "name": schema.name,
                                "description": schema.description,
                                "source": source::STORE,
                                "catalog_url": url,
                            }),
                            priority: priority::STORE,
                        },
                    ));
                }
                Err(error) => {
                    tracing::warn!(
                        %error,
                        schema_name = %schema.name,
                        source = %url,
                        "invalid glob pattern(s)"
                    );
                }
            }
        }
        Ok(())
    }

    /// Adds the schema from a `@jsonaschema` annotation in the root.
    pub fn add_from_document(&self, doc_url: &Url, root: &Node) {
        self.retain(|(rule, assoc)| match rule {
            AssociationRule::Url(u) => {
                !(u == doc_url && assoc.meta["source"] == source::SCHEMA_FIELD)
            }
            _ => true,
        });
        if let Some(url) = root
            .get(&KeyOrIndex::annotation(SCHEMA_KEY))
            .and_then(|v| v.as_string().cloned())
        {
            if let Some(url) = to_file_url(url.value(), &doc_url.to_file_path().unwrap_or_default())
            {
                self.associations.write().push((
                    AssociationRule::Url(doc_url.clone()),
                    SchemaAssociation {
                        url,
                        priority: priority::SCHEMA_FIELD,
                        meta: json!({ "source": source::SCHEMA_FIELD }),
                    },
                ));
            }
        }
    }

    pub fn add_from_config(&self, config: &Config) {
        for schema_rule in &config.rules {
            let file_rule = match schema_rule.file_rule.clone() {
                Some(rule) => rule,
                None => continue,
            };
            if let Some(url) = &schema_rule.url {
                self.associations.write().push((
                    file_rule.into(),
                    SchemaAssociation {
                        url: url.clone(),
                        meta: json!({
                            "source": source::CONFIG,
                        }),
                        priority: priority::CONFIG,
                    },
                ));
            }
        }
    }

    pub fn association_for(&self, file: &Url) -> Option<SchemaAssociation> {
        self.associations
            .read()
            .iter()
            .filter_map(|(rule, assoc)| {
                if rule.is_match(file.as_str()) {
                    Some(assoc.clone())
                } else {
                    None
                }
            })
            .max_by_key(|assoc| assoc.priority)
            .tap(|s| {
                if let Some(schema_association) = s {
                    tracing::debug!(
                        schema.url = %schema_association.url,
                        schema.name = schema_association.meta["name"].as_str().unwrap_or(""),
                        schema.source = schema_association.meta["source"].as_str().unwrap_or(""),
                        "found schema association"
                    );
                }
            })
    }

    async fn load_schemastore(&self, index_url: &Url) -> Result<SchemaStore, anyhow::Error> {
        if let Ok(s) = self.cache.load(index_url, false).await {
            return Ok(serde_json::from_value((*s).clone())?);
        }

        let schemastore = match self
            .fetch_external(index_url)
            .await
            .and_then(|v| serde_json::from_slice(&v).map_err(|e| anyhow!("{}", e)))
        {
            Ok(idx) => idx,
            Err(error) => {
                tracing::warn!(?error, "failed to load schemastore");
                if let Ok(s) = self.cache.load(index_url, true).await {
                    return Ok(serde_json::from_value((*s).clone())?);
                }
                return Err(error);
            }
        };

        if self.cache.is_cache_path_set() {
            if let Err(error) = self
                .cache
                .save(
                    index_url.clone(),
                    Arc::new(serde_json::to_value(&schemastore)?),
                )
                .await
            {
                tracing::warn!(%error, "failed to cache schemastore");
            }
        }

        Ok(schemastore)
    }

    async fn fetch_external(&self, file_url: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let _permit = self.concurrent_requests.acquire().await?;
        let data: Vec<u8> = match file_url.scheme() {
            "http" | "https" => self.env.fetch_file(file_url).await?,
            "file" => {
                self.env
                    .read_file(
                        self.env
                            .to_file_path(file_url)
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

#[derive(Clone)]
pub enum AssociationRule {
    Glob(GlobRule),
    Regex(Regex),
    Url(Url),
}

impl AssociationRule {
    pub fn glob(pattern: &str) -> Result<Self, anyhow::Error> {
        Ok(Self::Glob(GlobRule::new(&[pattern], &[] as &[&str])?))
    }

    pub fn regex(regex: &str) -> Result<Self, anyhow::Error> {
        Ok(Self::Regex(Regex::new(regex)?))
    }
}

impl From<Regex> for AssociationRule {
    fn from(v: Regex) -> Self {
        Self::Regex(v)
    }
}

impl From<GlobRule> for AssociationRule {
    fn from(v: GlobRule) -> Self {
        Self::Glob(v)
    }
}

impl AssociationRule {
    #[must_use]
    pub fn is_match(&self, text: &str) -> bool {
        match self {
            AssociationRule::Glob(g) => g.is_match(text),
            AssociationRule::Regex(r) => r.is_match(text),
            AssociationRule::Url(u) => u.as_str() == text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchemaAssociation {
    pub meta: Value,
    pub url: Url,
    pub priority: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchemaStore(Vec<SchemaStoreMeta>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaStoreMeta {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub url: Url,
    #[serde(default)]
    pub file_match: Vec<String>,
}
