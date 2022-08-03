use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use indexmap::IndexMap;
use jsona::dom::{visit_annotations, KeyOrIndex, Keys, Node};
use jsona_schema::from_node;
use jsonschema::{error::ValidationErrorKind, paths::JSONPointer, JSONSchema};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use jsona_schema::Schema;

pub struct JSONASchemaValidator {
    pub value: JSONSchema,
    pub annotations: IndexMap<String, JSONSchema>,
}

impl JSONASchemaValidator {
    pub fn new(schema: &JSONASchemaValue) -> Result<Self, Error> {
        let value = compile_json_schema(&schema.value, ".value")?;
        let mut annotations = IndexMap::default();
        if let Some(annotations_schemas) = schema.annotations.properties.as_ref() {
            for (key, value) in annotations_schemas.iter() {
                let key_value = format!("@{}", key);
                let annotation = compile_json_schema(value, &key_value)?;
                annotations.insert(key.to_string(), annotation);
            }
        }
        Ok(JSONASchemaValidator { value, annotations })
    }
    pub fn validate(&self, node: &Node) -> Vec<NodeValidationError> {
        let mut errors = vec![];
        jsona_schema_validate(&self.value, &mut errors, node, Keys::default());
        for (keys, annotation_node) in visit_annotations(node).into_iter() {
            if let Some(schema) = keys
                .last_annotation_key()
                .and_then(|v| self.annotations.get(&v.to_string()))
            {
                jsona_schema_validate(schema, &mut errors, &annotation_node, keys);
            }
        }
        errors
    }
}

fn compile_json_schema(schema: &Schema, key: &str) -> Result<JSONSchema, Error> {
    let json = serde_json::to_value(schema).map_err(|err| Error::ConvertJsonschema {
        key: key.into(),
        message: err.to_string(),
    })?;
    JSONSchema::options()
        .compile(&json)
        .map_err(|err| Error::ConvertJsonschema {
            key: key.into(),
            message: err.to_string(),
        })
}

fn jsona_schema_validate(
    schema: &JSONSchema,
    errors: &mut Vec<NodeValidationError>,
    node: &Node,
    keys: Keys,
) {
    let value = node.to_plain_json();
    if let Err(errs) = schema.validate(&value) {
        for err in errs {
            let info = err.to_string();
            if let Some(error) =
                NodeValidationError::new(node, keys.clone(), err.kind, err.instance_path, info)
            {
                errors.push(error);
            }
        }
    };
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct JSONASchemaValue {
    pub value: Box<Schema>,
    pub annotations: Box<Schema>,
}

impl JSONASchemaValue {
    pub fn from_node(node: Node) -> Result<Self, Error> {
        if node.validate().is_err() {
            return Err(Error::InvalidJsonaNode);
        };
        let value_node = node
            .try_get(&KeyOrIndex::property("value"))
            .map_err(|_| Error::InvalidNode(".value".into()))?;
        let value_schema = from_node(&value_node).map_err(|err| Error::InvalidSchemaValue {
            key: ".value".into(),
            message: err.to_string(),
        })?;
        let mut annotations_schemas: IndexMap<String, Schema> = Default::default();
        let annotations_value = node
            .try_get_as_object(&KeyOrIndex::property("annotations"))
            .map_err(|_| Error::InvalidNode(".annotations".into()))?;
        if let Some(annotations_value) = annotations_value {
            for (key, value) in annotations_value.value().read().kv_iter() {
                let key_value = format!("@{}", key.value());
                let schema = from_node(value).map_err(|err| Error::InvalidSchemaValue {
                    key: key_value.clone(),
                    message: err.to_string(),
                })?;
                annotations_schemas.insert(format!("@{}", key.value()), schema);
            }
        }
        Ok(JSONASchemaValue {
            value: value_schema.into(),
            annotations: Schema {
                schema_type: Some("object".into()),
                properties: Some(annotations_schemas),
                ..Default::default()
            }
            .into(),
        })
    }

    pub fn pointer(&self, keys: &Keys) -> Vec<&Schema> {
        let (annotation_key, keys) = keys.shift_annotation();
        let schema = match annotation_key {
            Some(key) => match self
                .annotations
                .properties
                .as_ref()
                .and_then(|v| v.get(key.value()))
            {
                Some(annotation_schema) => annotation_schema,
                None => {
                    if keys.is_empty() {
                        &self.annotations
                    } else {
                        return vec![];
                    }
                }
            },
            None => &self.value,
        };
        schema.pointer(&keys)
    }
}

impl FromStr for JSONASchemaValue {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let node: Node = s.parse().map_err(|_| Error::InvalidJsonaDoc)?;
        Self::from_node(node)
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
    #[error("invalid schema value at {key}")]
    InvalidSchemaValue { key: String, message: String },
    #[error("convert to convert jsonschema at {key}")]
    ConvertJsonschema { key: String, message: String },
}

/// A validation error that contains text ranges as well.
#[derive(Debug)]
pub struct NodeValidationError {
    pub keys: Keys,
    pub node: Node,
    pub kind: ValidationErrorKind,
    pub info: String,
}

impl NodeValidationError {
    fn new(
        node: &Node,
        keys: Keys,
        kind: ValidationErrorKind,
        instance_path: JSONPointer,
        info: String,
    ) -> Option<Self> {
        let mut keys = keys;
        let mut node = node.clone();

        'outer: for path in &instance_path {
            match path {
                jsonschema::paths::PathChunk::Property(p) => match &node {
                    Node::Object(t) => {
                        let entries = t.value().read();
                        for (k, entry) in entries.kv_iter() {
                            if k.value() == &**p {
                                keys = keys.join(k.into());
                                node = entry.clone();
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

impl Display for NodeValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        format!("{} at {}", self.info, self.keys).fmt(f)
    }
}
