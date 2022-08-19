use anyhow::anyhow;
use jsona::dom::{KeyOrIndex, Node};
use jsona_schema::{Schema, SchemaType};
use once_cell::sync::Lazy;
use parking_lot::{RwLock, RwLockReadGuard};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fmt::Debug, sync::Arc};
use url::Url;

use crate::{
    environment::Environment,
    schema::Fetcher,
    util::{url::is_url, GlobRule},
    HashMap,
};

pub const SCHEMA_REF_KEY: &str = "@jsonaschema";

pub static SCHEMA_REF_SCHEMA: Lazy<Schema> = Lazy::new(|| Schema {
    schema_type: Some(SchemaType::String.into()),
    description: Some("A ref to jsona schema".into()),
    ..Default::default()
});

static DEFAULT_SCHEMASTORE_URI: Lazy<Url> = Lazy::new(|| {
    Url::parse("https://cdn.jsdelivr.net/npm/@jsona/schemastore@latest/index.json").unwrap()
});

static RE_SCHEMA_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([A-Za-z_-]+)$").unwrap());

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
    pub const SCHEMA_FIELD: &str = "@jsonaschema";
}

#[derive(Clone)]
pub struct SchemaAssociations<E: Environment> {
    env: E,
    fetcher: Fetcher<E>,
    associations: Arc<RwLock<Vec<(AssociationRule, SchemaAssociation)>>>,
    cache: Arc<RwLock<HashMap<Url, Option<Arc<SchemaAssociation>>>>>,
    store_schema_urls: Arc<RwLock<HashMap<String, Url>>>,
}

impl<E: Environment> SchemaAssociations<E> {
    pub(crate) fn new(env: E, fetcher: Fetcher<E>) -> Self {
        Self {
            env,
            fetcher,
            associations: Default::default(),
            cache: Arc::new(RwLock::new(HashMap::default())),
            store_schema_urls: Arc::new(RwLock::new(HashMap::default())),
        }
    }
    pub fn add(&self, rule: AssociationRule, assoc: SchemaAssociation) {
        tracing::debug!("add an association {:?} {:?}", rule, assoc);
        self.associations.write().push((rule, assoc));
        self.cache.write().clear();
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Vec<(AssociationRule, SchemaAssociation)>> {
        self.associations.read()
    }

    pub fn clear(&self) {
        self.associations.write().clear();
        self.cache.write().clear();
        self.store_schema_urls.write().clear();
    }

    pub async fn add_from_schemastore(
        &self,
        url: &Option<Url>,
        base: &Option<Url>,
    ) -> Result<(), anyhow::Error> {
        let url = url.as_ref().unwrap_or(&DEFAULT_SCHEMASTORE_URI);
        let schemastore = self.load_schemastore(url).await?;
        tracing::info!(%url, "use schema store");
        for schema in &schemastore.0 {
            if self
                .store_schema_urls
                .write()
                .insert(schema.name.clone(), schema.url.clone())
                .is_some()
            {
                tracing::warn!("schema name {} already exist", schema.name);
            }
            let include = schema
                .file_match
                .iter()
                .filter_map(|v| GlobRule::preprocessing_pattern(v, base));
            match GlobRule::new(include, [] as [&str; 0]) {
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
    pub fn add_from_document(&self, doc_url: &Url, node: &Node) {
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
            self.cache.write().clear();
        }
        if let Some(url) = node
            .get(&KeyOrIndex::annotation(SCHEMA_REF_KEY))
            .and_then(|v| v.as_string().cloned())
            .and_then(|v| self.get_schema_url(v.value()))
        {
            self.add(
                AssociationRule::Url(doc_url.clone()),
                SchemaAssociation {
                    url,
                    priority: priority::SCHEMA_FIELD,
                    meta: json!({ "source": source::SCHEMA_FIELD }),
                },
            )
        }
    }

    pub fn get_schema_url(&self, schema_ref: &str) -> Option<Url> {
        if RE_SCHEMA_NAME.is_match(schema_ref) {
            self.store_schema_urls.read().get(schema_ref).cloned()
        } else {
            self.env.to_url(schema_ref)
        }
    }

    pub fn schema_key_complete_schema(&self) -> Schema {
        let enum_value: Vec<_> = self
            .store_schema_urls
            .read()
            .keys()
            .map(|v| json!(v))
            .collect();
        Schema {
            schema_type: Some(SchemaType::String.into()),
            enum_value: Some(enum_value),
            ..Default::default()
        }
    }

    pub fn query_for(&self, file: &Url) -> Option<Arc<SchemaAssociation>> {
        if let Some(assoc) = self.cache.read().get(file).cloned() {
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
        self.cache.write().insert(file.clone(), assoc.clone());
        assoc
    }

    async fn load_schemastore(&self, index_url: &Url) -> Result<SchemaStore, anyhow::Error> {
        self.fetcher
            .fetch(index_url)
            .await
            .and_then(|v| serde_json::from_slice(&v).map_err(|e| anyhow!("{}", e)))
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
            AssociationRule::Glob(glob) => write!(f, "Glob({:?})", glob),
            AssociationRule::Url(url) => write!(f, "Url({})", url),
        }
    }
}

impl From<GlobRule> for AssociationRule {
    fn from(v: GlobRule) -> Self {
        Self::Glob(v)
    }
}

impl From<Url> for AssociationRule {
    fn from(v: Url) -> Self {
        Self::Url(v)
    }
}

impl AssociationRule {
    pub fn batch(patterns: &[String], base: &Option<Url>) -> Result<Vec<Self>, anyhow::Error> {
        let mut rules = vec![];
        let mut glob_includes = vec![];
        for pattern in patterns {
            if is_url(pattern) {
                rules.push(Self::Url(pattern.parse()?));
            } else if let Some(p) = GlobRule::preprocessing_pattern(pattern, base) {
                glob_includes.push(p);
            }
        }
        if !glob_includes.is_empty() {
            rules.push(Self::Glob(GlobRule::new(&glob_includes, &[] as &[&str])?));
        }
        Ok(rules)
    }

    pub fn glob(pattern: &str) -> Result<Self, anyhow::Error> {
        Ok(Self::Glob(GlobRule::new(&[pattern], &[] as &[&str])?))
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! assert_association_rule {
        ($t:expr, $p:expr, $b:expr, $r:expr) => {
            let p: Vec<String> = $p.into_iter().map(|v| v.to_string()).collect();
            let t: Url = $t.parse().unwrap();
            let b: Option<Url> = $b.parse().ok();
            assert_eq!(
                AssociationRule::batch(&p, &b)
                    .unwrap()
                    .iter()
                    .any(|v| v.is_match(&t)),
                $r
            );
        };
    }

    #[test]
    fn test_association_rule() {
        assert_association_rule!("file:///home/u1/abc", ["abc"], "file:///home/u1", true);
        assert_association_rule!("file:///home/u1/abc", ["abc"], "file:///home/u1/", true);
        assert_association_rule!("file:///home/u1/abc", ["ab*"], "file:///home/u1", true);
        assert_association_rule!("file:///home/u1/abc", ["*bc"], "file:///home/u1", true);
        assert_association_rule!("file:///home/u1/abcd", ["ab*"], "file:///home/u1", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["abc"], "file:///home/u1", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["*abc"], "file:///home/u1", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["*/abc"], "file:///home/u1", true);
        assert_association_rule!(
            "file:///home/u1/p1/abc",
            ["**/abc"],
            "file:///home/u1",
            true
        );
        assert_association_rule!("file:///c%3A/abc", ["abc"], "file:///c%3A/abc", true);
        assert_association_rule!("file:///c%3A/abc", ["abc"], "file:///c%3A", true);
    }
}
