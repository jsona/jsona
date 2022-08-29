mod schema;

use indexmap::IndexMap;
use schema::OneOrMultiSchemas;
use std::{cell::RefCell, rc::Rc};
use thiserror::Error;

use jsona::dom::{KeyOrIndex, Keys, Node, ParseError};
use serde::de::DeserializeOwned;

pub use schema::{Schema, SchemaType};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("invalid jsona")]
    InvalidJsona(#[from] ParseError),
    #[error("conflict def {name}")]
    ConflictDef { keys: Keys, name: String },
    #[error("unknown ref {name}")]
    UnknownRef { keys: Keys, name: String },
    #[error("the type is unexpected")]
    UnexpectedType { keys: Keys },
    #[error("the schema type is not match value type")]
    UnmatchedSchemaType { keys: Keys },
    #[error("invalid schema value")]
    InvalidSchemaValue { keys: Keys, error: String },
    #[error("invalid compound value")]
    InvalidCompoundValue { keys: Keys },
}

pub fn from_str(value: &str) -> Result<Schema> {
    let node: Node = value.parse()?;
    from_node(&node)
}

pub fn from_node(node: &Node) -> Result<Schema> {
    let scope = Scope {
        node: node.clone(),
        keys: Default::default(),
        defs: Default::default(),
    };
    let mut schema = parse_node(scope.clone())?;
    let defs = scope.defs.take();
    if !defs.is_empty() {
        schema.defs = Some(defs);
    }
    Ok(schema)
}

#[derive(Debug, Clone)]
struct Scope {
    node: Node,
    keys: Keys,
    defs: Rc<RefCell<IndexMap<String, Schema>>>,
}

impl Scope {
    fn spawn(&self, key: impl Into<KeyOrIndex>, node: Node) -> Self {
        Self {
            node,
            keys: self.keys.clone().join(key.into()),
            defs: self.defs.clone(),
        }
    }
}

fn parse_node(scope: Scope) -> Result<Schema> {
    let mut def_value = String::new();
    if let Some(def) = parse_str_annotation(&scope, "@def")? {
        let mut defs = scope.defs.borrow_mut();
        if defs.contains_key(&def) {
            return Err(Error::ConflictDef {
                keys: scope.keys.clone(),
                name: def,
            });
        }
        defs.insert(def.clone(), Default::default());
        def_value = def;
    } else if let Some(ref_value) = parse_str_annotation(&scope, "@ref")? {
        let defs = scope.defs.borrow();
        if !defs.contains_key(&ref_value) {
            return Err(Error::UnknownRef {
                keys: scope.keys.clone(),
                name: ref_value,
            });
        }
        return Ok(Schema {
            ref_value: Some(format!("#/$defs/{}", ref_value)),
            ..Default::default()
        });
    }
    let mut schema: Schema = parse_object_annotation(&scope, "@schema")?.unwrap_or_default();
    schema.ref_value = None;
    if let Some(describe) = parse_str_annotation(&scope, "@describe")? {
        schema.description = Some(describe);
    }
    if exist_annotation(&scope, "@example") {
        schema.examples = Some(vec![scope.node.to_plain_json()])
    }
    if exist_annotation(&scope, "@default") {
        schema.default = Some(scope.node.to_plain_json())
    }
    let schema_types = schema.types();
    let node_type = SchemaType::from_node(&scope.node);
    if schema_types.is_empty() {
        schema.schema_type = node_type.map(Into::into);
    } else if let Some(node_type) = node_type {
        if !schema_types.contains(&node_type) {
            return Err(Error::UnmatchedSchemaType { keys: scope.keys });
        }
    }
    match &scope.node {
        Node::Object(obj) => {
            for (key, child) in obj.value().read().iter() {
                let child_scope = scope.spawn(key.clone(), child.clone());
                let key = key.value();
                let pattern = parse_str_annotation(&child_scope, "@pattern")?;
                let child_schema = parse_node(child_scope.clone())?;
                if let Some(pattern) = pattern {
                    let props = schema.pattern_properties.get_or_insert(Default::default());
                    props.insert(pattern, child_schema);
                } else {
                    let required = exist_annotation(&child_scope, "@required");
                    let props = schema.properties.get_or_insert(Default::default());
                    props.insert(key.to_string(), child_schema);
                    if required {
                        schema
                            .required
                            .get_or_insert(Default::default())
                            .push(key.to_string());
                    }
                }
            }
        }
        Node::Array(arr) => {
            let arr = arr.value().read();
            if arr.len() > 0 {
                let mut schemas = vec![];
                for (i, child) in arr.iter().enumerate() {
                    let child_scope = scope.spawn(i, child.clone());
                    schemas.push(parse_node(child_scope)?);
                }
                if let Some(compound) = parse_str_annotation(&scope, "@compound")? {
                    schema.schema_type = None;
                    match compound.as_str() {
                        "anyOf" => schema.any_of = Some(schemas),
                        "oneOf" => schema.one_of = Some(schemas),
                        "allOf" => schema.all_of = Some(schemas),
                        _ => {
                            return Err(Error::InvalidCompoundValue {
                                keys: scope.keys.join(KeyOrIndex::annotation("@compound")),
                            });
                        }
                    }
                } else if arr.len() == 1 {
                    schema.items = Some(OneOrMultiSchemas::new(schemas));
                } else {
                    schema.items = Some(OneOrMultiSchemas::new(schemas))
                }
            }
        }
        _ => {}
    }
    if exist_annotation(&scope, "@anytype") {
        schema.schema_type = None;
    }
    if !def_value.is_empty() {
        scope.defs.borrow_mut().insert(def_value.clone(), schema);
        return Ok(Schema {
            ref_value: Some(format!("#/$defs/{}", def_value)),
            ..Default::default()
        });
    }
    Ok(schema)
}

fn exist_annotation(scope: &Scope, name: &str) -> bool {
    scope.node.get(&KeyOrIndex::annotation(name)).is_some()
}

fn parse_object_annotation<T: DeserializeOwned>(scope: &Scope, name: &str) -> Result<Option<T>> {
    match scope.node.get_as_object(name) {
        Some((key, Some(value))) => {
            let value = Node::from(value).to_plain_json();
            match serde_json::from_value(value) {
                Ok(v) => Ok(Some(v)),
                Err(err) => Err(Error::InvalidSchemaValue {
                    keys: scope.keys.clone().join(key),
                    error: err.to_string(),
                }),
            }
        }
        Some((key, None)) => Err(Error::UnexpectedType {
            keys: scope.keys.clone().join(key),
        }),
        None => Ok(None),
    }
}

fn parse_str_annotation(scope: &Scope, name: &str) -> Result<Option<String>> {
    match scope.node.get_as_string(name) {
        Some((_, Some(value))) => Ok(Some(value.value().to_string())),
        Some((key, None)) => Err(Error::UnexpectedType {
            keys: scope.keys.clone().join(key),
        }),
        None => Ok(None),
    }
}
