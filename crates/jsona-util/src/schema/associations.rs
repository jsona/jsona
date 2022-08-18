use anyhow::anyhow;
use jsona::dom::{KeyOrIndex, Node};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fmt::Debug, path::Path, sync::Arc};
use url::Url;

use crate::{
    environment::Environment,
    schema::Fetcher,
    util::{
        is_url,
        path_utils::{remove_tail_slash, to_unix},
        to_file_uri, GlobRule,
    },
    HashMap,
};

static DEFAULT_SCHEMASTORE_URI: Lazy<Url> = Lazy::new(|| {
    Url::parse("https://cdn.jsdelivr.net/npm/@jsona/schemastore@latest/index.json").unwrap()
});

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
        tracing::debug!("add an association {:?} {:?}", rule, assoc);
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

    pub async fn add_from_schemastore(
        &self,
        url: &Option<Url>,
        base: &Path,
    ) -> Result<(), anyhow::Error> {
        let url = url.as_ref().unwrap_or(&DEFAULT_SCHEMASTORE_URI);
        let base = to_unix(remove_tail_slash(base.display().to_string()));
        let schemastore = self.load_schemastore(url).await?;
        for schema in &schemastore.0 {
            let include = schema
                .file_match
                .iter()
                .map(|v| GlobRule::preprocessing_pattern(v, &base));
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
    pub fn batch(patterns: &[String], base: &Path) -> Result<Vec<Self>, anyhow::Error> {
        let base = to_unix(remove_tail_slash(base.display().to_string()));
        let mut rules = vec![];
        let mut glob_includes = vec![];
        for pattern in patterns {
            if is_url(pattern) {
                rules.push(Self::Url(pattern.parse()?));
            } else {
                glob_includes.push(GlobRule::preprocessing_pattern(pattern, &base));
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
    use std::path::PathBuf;
    macro_rules! assert_association_rule {
        ($t:expr, $p:expr, $b:expr, $r:expr) => {
            let p: Vec<String> = $p.into_iter().map(|v| v.to_string()).collect();
            let t: Url = $t.parse().unwrap();
            assert_eq!(
                AssociationRule::batch(&p, &PathBuf::from($b))
                    .unwrap()
                    .iter()
                    .any(|v| v.is_match(&t)),
                $r
            );
        };
    }

    #[test]
    fn test_association_rule() {
        assert_association_rule!("file:///home/u1/abc", ["abc"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/abc", ["abc"], "/home/u1/", true);
        assert_association_rule!("file:///home/u1/abc", ["ab*"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/abc", ["*bc"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/abcd", ["ab*"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/abcd", ["ab*"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["abc"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["*abc"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["*/abc"], "/home/u1", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["**/abc"], "/home/u1", true);
        assert_association_rule!("file:///c%3A/abc", ["abc"], "C:\\abc", true);
        assert_association_rule!("file:///c%3A/abc", ["abc"], "C:\\abc\\", true);
        assert_association_rule!("file:///home/u1/p1/abc", ["/abc"], "/home/u1", false);
        assert_association_rule!("file:///home/u1/p1/abc", ["/abc"], "/home/u1/p1", true);
    }
}
