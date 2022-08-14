use anyhow::anyhow;
use jsona::dom::{KeyOrIndex, Node};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fmt::Debug, sync::Arc};
use url::Url;

use crate::{
    config::Config,
    environment::Environment,
    schema::Fetcher,
    util::{to_file_uri, GlobRule},
    HashMap,
};

pub const DEFAULT_SCHEMASTORE: &str =
    "https://cdn.jsdelivr.net/npm/@jsona/schemastore@latest/index.json";

pub const SCHEMA_KEY: &str = "@jsonaschema";

pub mod priority {
    pub const STORE: usize = 30;
    pub const CONFIG: usize = 50;
    pub const LSP_CONFIG: usize = 70;
    pub const SCHEMA_FIELD: usize = 80;
    pub const MAX: usize = usize::MAX;
}

pub mod source {
    pub const STORE: &str = "store";
    pub const CONFIG: &str = "config";
    pub const LSP_CONFIG: &str = "lsp_config";
    pub const SCHEMA_FIELD: &str = "$schema";
}

#[derive(Clone)]
pub struct SchemaAssociations<E: Environment> {
    env: E,
    fetcher: Fetcher<E>,
    associations: Arc<RwLock<Vec<(AssociationRule, SchemaAssociation)>>>,
    cache: Arc<Mutex<HashMap<Url, Option<Arc<SchemaAssociation>>>>>,
}

impl<E: Environment> SchemaAssociations<E> {
    pub(crate) fn new(env: E, fetcher: Fetcher<E>) -> Self {
        Self {
            env,
            fetcher,
            associations: Default::default(),
            cache: Arc::new(Mutex::new(HashMap::default())),
        }
    }
    pub fn add(&self, rule: AssociationRule, assoc: SchemaAssociation) {
        self.associations.write().push((rule, assoc));
        self.cache.lock().clear();
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Vec<(AssociationRule, SchemaAssociation)>> {
        self.associations.read()
    }

    pub fn clear(&self) {
        self.associations.write().clear();
        self.cache.lock().clear();
    }

    pub async fn add_from_schemastore(&self, url: &Url) -> Result<(), anyhow::Error> {
        let schemastore = self.load_schemastore(url).await?;
        for schema in &schemastore.0 {
            match GlobRule::new(&schema.file_match, [] as [&str; 0]) {
                Ok(rule) => {
                    self.add(
                        rule.into(),
                        SchemaAssociation {
                            url: schema.url.clone(),
                            meta: json!({
                                "name": schema.name,
                                "description": schema.description,
                                "source": source::STORE,
                            }),
                            priority: priority::STORE,
                        },
                    );
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
        let mut dirty = false;
        self.associations
            .write()
            .retain(|(rule, assoc)| match rule {
                AssociationRule::Url(u) => {
                    if u == doc_url && assoc.meta["source"] == source::SCHEMA_FIELD {
                        dirty = true;
                        false
                    } else {
                        true
                    }
                }
                _ => true,
            });
        if dirty {
            self.cache.lock().clear();
        }
        if let Some(url) = root
            .get(&KeyOrIndex::annotation(SCHEMA_KEY))
            .and_then(|v| v.as_string().cloned())
        {
            if let Some(url) = to_file_uri(url.value(), &self.env.cwd()) {
                self.add(
                    AssociationRule::Url(doc_url.clone()),
                    SchemaAssociation {
                        url,
                        priority: priority::SCHEMA_FIELD,
                        meta: json!({ "source": source::SCHEMA_FIELD }),
                    },
                );
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
                self.add(
                    file_rule.into(),
                    SchemaAssociation {
                        url: url.clone(),
                        meta: json!({
                            "source": source::CONFIG,
                        }),
                        priority: priority::CONFIG,
                    },
                );
            }
        }
    }

    pub fn query_for(&self, file: &Url) -> Option<Arc<SchemaAssociation>> {
        if let Some(assoc) = self.cache.lock().get(file).cloned() {
            return assoc;
        }
        let assoc = self
            .associations
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
            .map(Arc::new);
        self.cache.lock().insert(file.clone(), assoc.clone());
        assoc
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
    Url(Url),
}

impl Debug for AssociationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssociationRule::Glob(_) => f.write_str("Glob"),
            AssociationRule::Url(url) => write!(f, "Url({})", url),
        }
    }
}

impl AssociationRule {
    pub fn glob(pattern: &str) -> Result<Self, anyhow::Error> {
        Ok(Self::Glob(GlobRule::new(&[pattern], &[] as &[&str])?))
    }
}

impl From<GlobRule> for AssociationRule {
    fn from(v: GlobRule) -> Self {
        Self::Glob(v)
    }
}

impl From<&Url> for AssociationRule {
    fn from(v: &Url) -> Self {
        Self::Url(v.clone())
    }
}

impl AssociationRule {
    #[must_use]
    pub fn is_match(&self, url: &Url) -> bool {
        match self {
            AssociationRule::Glob(g) => g.is_match_url(url),
            AssociationRule::Url(u) => u == url,
        }
    }
}

#[derive(Clone)]
pub struct SchemaAssociation {
    pub meta: Value,
    pub url: Url,
    pub priority: usize,
}

impl Debug for SchemaAssociation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaAssociation")
            .field(
                "meta",
                &serde_json::to_string(&self.meta).unwrap_or_default(),
            )
            .field("url", &self.url.to_string())
            .field("priority", &self.priority)
            .finish()
    }
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
