mod validates;

use std::{collections::HashSet, str::FromStr};

use jsona::dom::{visit_annotations, KeyOrIndex, Keys, Node};
use jsona_schema::from_node;
use thiserror::Error;
pub use validates::Error as JSONASchemaValidationError;

pub use jsona_schema::Schema;

pub const ANNOATION_KEY: &str = "annotations";
pub const VALUE_KEY: &str = "value";

pub struct JSONASchemaValidator {
    schema: Schema,
    annotation_names: HashSet<String>,
}

impl JSONASchemaValidator {
    pub fn from_node(node: &Node) -> Result<Self, Error> {
        if node.validate().is_err() {
            return Err(Error::InvalidJsonaNode);
        };
        let mut schema = from_node(node).map_err(|err| Error::InvalidSchema(err.to_string()))?;
        let mut annotation_names = HashSet::default();
        if let Some(properties) = schema
            .properties
            .as_mut()
            .and_then(|v| v.get_mut(ANNOATION_KEY))
            .and_then(|v| v.properties.as_mut())
        {
            let keys: Vec<String> = properties.keys().cloned().collect();
            for key in keys.iter() {
                if let Some(value) = properties.remove(key) {
                    let new_key = format!("@{}", key);
                    properties.insert(new_key.clone(), value);
                    annotation_names.insert(new_key);
                }
            }
        }
        Ok(Self {
            schema,
            annotation_names,
        })
    }

    pub fn validate(&self, node: &Node) -> Vec<JSONASchemaValidationError> {
        let mut collect_errors = vec![];
        let default_defs = Default::default();
        let defs = self.schema.defs.as_ref().unwrap_or(&default_defs);
        if let Some(value_schema) = self
            .schema
            .properties
            .as_ref()
            .and_then(|v| v.get(VALUE_KEY))
        {
            collect_errors.extend(validates::validate(
                defs,
                value_schema,
                &Keys::default(),
                node,
            ));
        }
        for (keys, value) in visit_annotations(node).into_iter() {
            if let Some(key) = keys.last_annotation_key() {
                if let Some(schema) = self.schema.properties.as_ref().and_then(|v| {
                    v.get(ANNOATION_KEY)
                        .and_then(|s| s.properties.as_ref())
                        .and_then(|p| p.get(key.value()))
                }) {
                    collect_errors.extend(validates::validate(defs, schema, &keys, &value));
                }
            }
        }
        collect_errors
    }

    pub fn pointer(&self, keys: &Keys) -> Vec<&Schema> {
        let (annotation_key, keys) = keys.shift_annotation();
        let new_keys = match annotation_key {
            Some(key) => {
                let mut anno_keys = vec![KeyOrIndex::property(ANNOATION_KEY)];
                if self.annotation_names.contains(key.value()) {
                    anno_keys.push(KeyOrIndex::property(key.value()));
                }
                let new_keys = Keys::new(anno_keys.into_iter());
                new_keys.extend(keys)
            }
            None => {
                let new_keys = Keys::new([KeyOrIndex::property(VALUE_KEY)].into_iter());
                new_keys.extend(keys)
            }
        };
        self.schema.pointer(&new_keys)
    }
}

impl FromStr for JSONASchemaValidator {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let node: Node = s.parse().map_err(|_| Error::InvalidJsonaDoc)?;
        Self::from_node(&node)
    }
}

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid jsona doc")]
    InvalidJsonaDoc,
    #[error("invalid jsona node")]
    InvalidJsonaNode,
    #[error("invalid node at {0}")]
    InvalidNode(String),
    #[error("invalid schema {0}")]
    InvalidSchema(String),
}
