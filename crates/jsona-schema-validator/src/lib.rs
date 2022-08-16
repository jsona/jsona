use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
    str::FromStr,
};

use jsona::dom::{visit_annotations, KeyOrIndex, Keys, Node};
use jsona_schema::from_node;
use jsonschema::{error::ValidationErrorKind, paths::JSONPointer, JSONSchema, ValidationError};
use serde_json::json;
use thiserror::Error;

pub use jsona_schema::Schema;

pub const ANNOATION_KEY: &str = "annotations";
pub const VALUE_KEY: &str = "value";

pub struct JSONASchemaValidator {
    jsonschema: JSONSchema,
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
        let json =
            serde_json::to_value(&schema).map_err(|err| Error::InvalidSchema(err.to_string()))?;
        let jsonschema = JSONSchema::options()
            .compile(&json)
            .map_err(|err| Error::InvalidSchema(err.to_string()))?;
        Ok(Self {
            jsonschema,
            schema,
            annotation_names,
        })
    }

    pub fn validate(&self, node: &Node) -> Vec<JSONASchemaValidationError> {
        let mut collect_errors = vec![];
        let value = json!({
            VALUE_KEY: node.to_plain_json(),
        });
        if let Err(errors) = self.jsonschema.validate(&value) {
            JSONASchemaValidationError::batch(
                &mut collect_errors,
                errors.collect(),
                node,
                Keys::default(),
            )
        }
        for (keys, annotation_node) in visit_annotations(node).into_iter() {
            if let Some(key) = keys.last_annotation_key() {
                let value = json!({
                    ANNOATION_KEY: {
                        key.value(): annotation_node.to_plain_json(),
                    },
                });
                let validate_result = self.jsonschema.validate(&value);
                if let Err(errors) = validate_result {
                    JSONASchemaValidationError::batch(
                        &mut collect_errors,
                        errors.collect(),
                        &annotation_node,
                        keys,
                    )
                }
            }
        }
        collect_errors
    }

    pub fn pointer(&self, keys: &Keys) -> Vec<&Schema> {
        let (annotation_key, keys) = keys.shift_annotation();
        let new_keys = match annotation_key {
            Some(key) => {
                let mut key_items = vec![KeyOrIndex::property(ANNOATION_KEY)];
                if self.annotation_names.contains(key.value()) {
                    key_items.push(KeyOrIndex::property(key.value()));
                }
                let new_keys = Keys::new(key_items.into_iter());
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

/// A validation error that contains text ranges as well.
#[derive(Debug)]
pub struct JSONASchemaValidationError {
    pub keys: Keys,
    pub node: Node,
    pub kind: ValidationErrorKind,
    pub info: String,
}

impl JSONASchemaValidationError {
    fn batch(collectors: &mut Vec<Self>, errors: Vec<ValidationError>, node: &Node, keys: Keys) {
        for err in errors {
            let info = err.to_string();
            if let Some(error) = Self::new(node, keys.clone(), err.kind, err.instance_path, info) {
                collectors.push(error);
            }
        }
    }
    fn new(
        node: &Node,
        keys: Keys,
        kind: ValidationErrorKind,
        instance_path: JSONPointer,
        info: String,
    ) -> Option<Self> {
        let mut keys = keys;
        let mut node = node.clone();

        'outer: for path in instance_path.iter().skip(1) {
            match path {
                jsonschema::paths::PathChunk::Property(p) => match &node {
                    Node::Object(t) => {
                        let entries = t.value().read();
                        for (k, v) in entries.kv_iter() {
                            if k.value() == &**p {
                                keys = keys.join(k.into());
                                node = v.clone();
                                continue 'outer;
                            }
                        }
                        break 'outer;
                    }
                    _ => break 'outer,
                },
                jsonschema::paths::PathChunk::Index(idx) => {
                    node = node.try_get(&KeyOrIndex::Index(*idx)).ok()?;
                    keys = keys.join((*idx).into());
                }
                jsonschema::paths::PathChunk::Keyword(_) => {}
            }
        }

        Some(Self {
            keys,
            node,
            kind,
            info,
        })
    }
}

impl Display for JSONASchemaValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        format!("{} at {}", self.info, self.keys).fmt(f)
    }
}
