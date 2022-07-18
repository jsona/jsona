mod error;
mod schema;

use indexmap::IndexMap;
use std::{cell::RefCell, rc::Rc};

use jsona::{
    dom::{KeyOrIndex, Keys, Node},
    parser,
};
use serde::de::DeserializeOwned;

pub use error::Error;
pub use schema::Schema;
pub type Result<T> = std::result::Result<T, Error>;

pub fn from_str(value: &str) -> Result<Schema> {
    let parse = parser::parse(value);
    if !parse.errors.is_empty() {
        return Err(Error::Syntax(parse.errors.into_iter().collect()));
    }
    let node = parse.into_dom();
    from_node(node)
}

pub fn from_node(node: Node) -> Result<Schema> {
    if let Err(errors) = node.validate() {
        return Err(Error::Dom(errors.collect()));
    }
    let scope = Scope {
        node,
        keys: Default::default(),
        defs: Default::default(),
    };
    let mut schema = parse_value(scope.clone())?;
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
    fn spwan(&self, key: KeyOrIndex, node: Node) -> Self {
        Self {
            node,
            keys: self.keys.clone().join(key),
            defs: self.defs.clone(),
        }
    }
}

fn parse_value(scope: Scope) -> Result<Schema> {
    let mut def_value = String::new();
    if let Some(def) = parse_str_annotation(&scope, "def")? {
        let mut defs = scope.defs.borrow_mut();
        if defs.contains_key(&def) {
            return Err(Error::ConflictDef(def));
        }
        defs.insert(def.clone(), Default::default());
        def_value = def;
    } else if let Some(ref_value) = parse_str_annotation(&scope, "ref")? {
        let defs = scope.defs.borrow();
        if !defs.contains_key(&ref_value) {
            return Err(Error::MissedDef(ref_value));
        }
        return Ok(Schema {
            ref_value: Some(format!("#/defs/{}", ref_value)),
            ..Default::default()
        });
    }
    let mut schema: Schema = parse_object_annotation(&scope, "schema")?.unwrap_or_default();
    if let Some(desc) = parse_str_annotation(&scope, "desc")? {
        schema.description = Some(desc);
    }
    if schema.schema_type.is_none() {
        let schema_type = match &scope.node {
            Node::Null(_) => "null",
            Node::Bool(_) => "boolean",
            Node::Number(_) => "number",
            Node::String(_) => "string",
            Node::Array(_) => "array",
            Node::Object(_) => "object",
        };
        schema.schema_type = Some(schema_type.to_string());
    }
    let schema_type = schema.schema_type.as_ref().unwrap();
    if schema_type == "object" {
        if let Node::Object(obj) = &scope.node {
            for (key, child) in obj.value().read().iter() {
                let child_scope = scope.spwan(KeyOrIndex::PropertyKey(key.clone()), child.clone());
                let key = key.value();
                let pattern = parse_str_annotation(&child_scope, "pattern")?;
                let optional = exist_annotation(&child_scope, "optional");
                let child_schema = parse_value(child_scope.clone())?;
                if let Some(pattern) = pattern {
                    let props = schema.pattern_properties.get_or_insert(Default::default());
                    if props.contains_key(key) {
                        return Err(Error::Conflict {
                            keys: child_scope.keys.join(KeyOrIndex::annotation("pattern")),
                        });
                    }
                    props.insert(pattern, child_schema);
                } else {
                    let props = schema.properties.get_or_insert(Default::default());
                    props.insert(key.to_string(), child_schema);
                    if !optional {
                        schema
                            .required
                            .get_or_insert(Default::default())
                            .push(key.to_string());
                    }
                }
            }
        } else {
            return Err(Error::MismatchType {
                keys: scope.keys.clone(),
            });
        }
    } else if schema_type == "array" && scope.node.is_array() {
        if let Node::Array(arr) = &scope.node {
            let arr = arr.value().read();
            if arr.len() > 0 && schema.items.is_none() {
                let compound = parse_str_annotation(&scope, "compound")?;
                match compound {
                    Some(compound) => {
                        let mut schemas = vec![];
                        for (i, child) in arr.iter().enumerate() {
                            let child_scope = scope.spwan(KeyOrIndex::Index(i), child.clone());
                            schemas.push(parse_value(child_scope)?);
                        }
                        match compound.as_str() {
                            "allOf" => schema.all_of = Some(schemas),
                            "anyOf" => schema.any_of = Some(schemas),
                            "oneOf" => schema.one_of = Some(schemas),
                            _ => {
                                return Err(Error::InvalidValue {
                                    keys: scope.keys.join(KeyOrIndex::annotation("compound")),
                                })
                            }
                        }
                    }
                    None => {
                        let child_scope = scope.spwan(KeyOrIndex::Index(0), arr[0].clone());
                        schema.items = Some(parse_value(child_scope)?.into())
                    }
                }
            }
        } else {
            return Err(Error::MismatchType {
                keys: scope.keys.clone(),
            });
        }
    }
    if !def_value.is_empty() {
        scope.defs.borrow_mut().insert(def_value.clone(), schema);
        return Ok(Schema {
            ref_value: Some(format!("#/defs/{}", def_value)),
            ..Default::default()
        });
    }
    Ok(schema)
}

fn exist_annotation(scope: &Scope, name: &str) -> bool {
    scope.node.get_annotation(name).is_some()
}

fn parse_object_annotation<T: DeserializeOwned>(scope: &Scope, name: &str) -> Result<Option<T>> {
    let value = scope
        .node
        .get_as_object(&KeyOrIndex::annotation(name))
        .map_err(|_| Error::MismatchType {
            keys: scope.keys.clone().join(KeyOrIndex::annotation(name)),
        })?;
    if let Some(value) = value {
        let value: Node = value.into();
        let value = value.to_plain_json();
        let schema = serde_json::from_value(value).map_err(|_| Error::MismatchType {
            keys: scope
                .keys
                .clone()
                .join(KeyOrIndex::AnnotationKey(name.into())),
        })?;
        Ok(Some(schema))
    } else {
        Ok(None)
    }
}

fn parse_str_annotation(scope: &Scope, name: &str) -> Result<Option<String>> {
    let value = scope
        .node
        .get_as_string(&KeyOrIndex::annotation(name))
        .map_err(|_| Error::MismatchType {
            keys: scope.keys.clone().join(KeyOrIndex::annotation(name)),
        })?;
    Ok(value.map(|v| v.value().to_string()))
}
