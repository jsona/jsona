use jsona::dom::{KeyOrIndex, Node};
use parking_lot::{RwLock, RwLockReadGuard};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::Arc;
use tap::Tap;
use url::Url;

use crate::{config::Config, util::GlobRule};

pub const SCHEMA_KEY: &str = "__schema__";

pub mod priority {
    pub const CONFIG: usize = 50;
    pub const LSP_CONFIG: usize = 60;
    pub const DIRECTIVE: usize = 75;
    pub const MAX: usize = usize::MAX;
}

pub mod source {
    pub const CONFIG: &str = "config";
    pub const LSP_CONFIG: &str = "lsp_config";
    pub const MANUAL: &str = "manual";
    pub const SCHEMA_FIELD: &str = "$schema";
    pub const DIRECTIVE: &str = "directive";
}

#[derive(Clone, Default)]
pub struct SchemaAssociations {
    associations: Arc<RwLock<Vec<(AssociationRule, SchemaAssociation)>>>,
}

impl SchemaAssociations {
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

    /// Adds the schema from either a directive, or a `@schema` key in the root.
    pub fn add_from_document(&self, doc_url: &Url, root: &Node) {
        self.retain(|(rule, assoc)| match rule {
            AssociationRule::Url(u) => !(u == doc_url && assoc.meta["source"] == source::DIRECTIVE),
            _ => true,
        });
        if let Some(url) = root
            .get(&KeyOrIndex::annotation(SCHEMA_KEY))
            .and_then(|v| v.as_string().cloned())
        {
            let url_value = url.value();
            let schema_url = if url_value.starts_with('.') {
                match doc_url.join(url_value) {
                    Ok(s) => Some(s),
                    Err(error) => {
                        tracing::error!(%error, "invalid schema directive");
                        None
                    }
                }
            } else {
                match url_value.parse() {
                    Ok(s) => Some(s),
                    Err(error) => {
                        tracing::error!(%error, "invalid schema directive");
                        None
                    }
                }
            };
            if let Some(url) = schema_url {
                self.associations.write().push((
                    AssociationRule::Url(doc_url.clone()),
                    SchemaAssociation {
                        url,
                        priority: priority::DIRECTIVE,
                        meta: json!({ "source": source::DIRECTIVE }),
                    },
                ));
            }
        }
    }

    pub fn add_from_config(&self, config: &Config) {
        for schema_opts in &config.schemas {
            let file_rule = match schema_opts.file_rule.clone() {
                Some(rule) => rule,
                None => continue,
            };
            if let Some(url) = &schema_opts.url {
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
