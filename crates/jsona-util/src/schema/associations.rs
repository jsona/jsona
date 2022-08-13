use anyhow::anyhow;
use jsona::dom::{KeyOrIndex, Node};
use parking_lot::{RwLock, RwLockReadGuard};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tap::Tap;
use url::Url;

use crate::{
    config::Config,
    environment::Environment,
    schema::Fetcher,
    util::{path_utils::to_unix, to_file_path, to_file_url, GlobRule},
};

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
    env: E,
    fetcher: Fetcher<E>,
    associations: Arc<RwLock<Vec<(AssociationRule, SchemaAssociation)>>>,
}

impl<E: Environment> SchemaAssociations<E> {
    pub(crate) fn new(env: E, fetcher: Fetcher<E>) -> Self {
        Self {
            env,
            fetcher,
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
            if let Some(url) = to_file_url(url.value(), &self.env.cwd()) {
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
                if rule.is_match(file) {
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
        let schemastore = match self
            .fetcher
            .fetch(index_url)
            .await
            .and_then(|v| serde_json::from_slice(&v).map_err(|e| anyhow!("{}", e)))
        {
            Ok(idx) => idx,
            Err(error) => {
                tracing::warn!(?error, "failed to load schemastore");
                return Err(error);
            }
        };

        Ok(schemastore)
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
    pub fn is_match(&self, url: &Url) -> bool {
        match self {
            AssociationRule::Glob(g) => to_file_path(url)
                .map(|v| g.is_match(to_unix(v)))
                .unwrap_or_default(),
            AssociationRule::Regex(r) => r.is_match(url.as_str()),
            AssociationRule::Url(u) => u == url,
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
