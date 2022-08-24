use std::fmt::{Display, Formatter};

use either::Either;
use fancy_regex::Regex;
use jsona::dom::{KeyOrIndex, Keys, Node};
use jsona_schema::{resolve, Schema, SchemaType};

pub fn validate(schema: &Schema, node: &Node) -> Vec<Error> {
    let mut errors = vec![];
    validate_impl(&mut errors, schema, schema, &Keys::default(), node);
    errors
}

fn validate_impl(
    errors: &mut Vec<Error>,
    root_schema: &Schema,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    validate_type(errors, root_schema, local_schema, keys, node);
    validate_properties(errors, root_schema, local_schema, keys, node);
    validate_items(errors, root_schema, local_schema, keys, node);
    validate_enum(errors, root_schema, local_schema, keys, node)
}

fn validate_type(
    errors: &mut Vec<Error>,
    _root_schema: &Schema,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let types = local_schema.types();
    if types.is_empty() {
        return;
    }
    let mut is_type_match = false;
    for schema_type in types.iter() {
        if schema_type.match_node(node) {
            is_type_match = true;
            break;
        }
    }
    if !is_type_match {
        errors.push(Error::new(
            keys,
            node,
            ErrorKind::Type {
                types: types.into_iter().collect(),
            },
        ));
    }
}

fn validate_properties(
    errors: &mut Vec<Error>,
    root_schema: &Schema,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let object = match node.as_object() {
        Some(v) => v,
        None => return,
    };
    for (key, value) in object.value().read().kv_iter() {
        let new_keys = keys.join(KeyOrIndex::property(key.value()));
        let is_property_passed = if let Some(schema) = local_schema
            .properties
            .as_ref()
            .and_then(|v| v.get(key.value()))
        {
            if let Some(schema) = resolve(root_schema, schema) {
                validate_impl(errors, root_schema, schema, &new_keys, value);
            }
            true
        } else {
            false
        };
        let mut is_pattern_passed = false;
        if let Some(patterns) = local_schema.pattern_properties.as_ref() {
            for (pat, schema) in patterns.iter() {
                if let Ok(re) = Regex::new(pat) {
                    if let Ok(true) = re.is_match(key.value()) {
                        if let Some(schema) = resolve(root_schema, schema) {
                            validate_impl(errors, root_schema, schema, &new_keys, value);
                            is_pattern_passed = true;
                        }
                    }
                }
            }
        }
        if is_property_passed || is_pattern_passed {
            continue;
        }

        if let Some(additional_properties) = local_schema.additional_properties.as_ref() {
            match additional_properties.value.as_ref() {
                Either::Left(allowed) => {
                    if !allowed {
                        errors.push(Error::new(
                            keys,
                            value,
                            ErrorKind::AdditionalProperties {
                                key: key.value().to_string(),
                            },
                        ));
                    }
                }
                Either::Right(schema) => {
                    if let Some(schema) = resolve(root_schema, schema) {
                        validate_impl(errors, root_schema, schema, &new_keys, value)
                    }
                }
            }
        }
    }
}

fn validate_items(
    errors: &mut Vec<Error>,
    root_schema: &Schema,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    let array = match node.as_array() {
        Some(v) => v,
        None => return,
    };
    if let Some(items) = local_schema.items.as_ref() {
        match items.value.as_ref() {
            Either::Left(schema) => {
                if let Some(schema) = resolve(root_schema, schema) {
                    for (idx, value) in array.value().read().iter().enumerate() {
                        let new_keys = keys.join(KeyOrIndex::Index(idx));
                        validate_impl(errors, root_schema, schema, &new_keys, value);
                    }
                }
            }
            Either::Right(schemas) => {
                let items = array.value().read();
                for (idx, (value, schema)) in items.iter().zip(schemas.iter()).enumerate() {
                    if let Some(schema) = resolve(root_schema, schema) {
                        let new_keys = keys.join(KeyOrIndex::Index(idx));
                        validate_impl(errors, root_schema, schema, &new_keys, value);
                    }
                }
                let schemas_len = schemas.len();
                if items.len() > schemas_len {
                    if let Some(additional_items) = local_schema.additional_items.as_ref() {
                        match additional_items.value.as_ref() {
                            Either::Left(allowed) => {
                                if !allowed {
                                    errors.push(Error::new(keys, node, ErrorKind::AdditionalItems));
                                }
                            }
                            Either::Right(schema) => {
                                if let Some(schema) = resolve(root_schema, schema) {
                                    for (idx, value) in items.iter().skip(schemas_len).enumerate() {
                                        let new_keys =
                                            keys.join(KeyOrIndex::Index(idx + schemas_len));
                                        validate_impl(
                                            errors,
                                            root_schema,
                                            schema,
                                            &new_keys,
                                            value,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn validate_enum(
    errors: &mut Vec<Error>,
    _root_schema: &Schema,
    local_schema: &Schema,
    keys: &Keys,
    node: &Node,
) {
    if let Some(enum_items) = local_schema.enum_value.as_ref() {
        let value = node.to_plain_json();
        let mut contains = false;
        for enum_value in enum_items.iter() {
            if is_matching(&value, enum_value) {
                contains = true;
                break;
            }
        }
        if !contains {
            errors.push(Error::new(keys, node, ErrorKind::Enum));
        }
    }
}

pub struct Error {
    pub keys: Keys,
    pub node: Node,
    pub kind: ErrorKind,
}

impl Error {
    pub fn new(keys: &Keys, node: &Node, kind: ErrorKind) -> Self {
        Self {
            keys: keys.clone(),
            node: node.clone(),
            kind,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.kind, self.keys)
    }
}

pub enum ErrorKind {
    Type { types: Vec<SchemaType> },
    AdditionalProperties { key: String },
    AdditionalItems,
    Enum,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Type { types } => write!(
                f,
                "The value must be any of: {}",
                types
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            ErrorKind::AdditionalProperties { key } => {
                write!(f, "Additional property '{}' is not allowed", key)
            }
            ErrorKind::AdditionalItems => write!(f, "Additional items are not allowed"),
            ErrorKind::Enum => write!(f, "Enum conditions are not met"),
        }
    }
}

fn is_matching(va: &serde_json::Value, vb: &serde_json::Value) -> bool {
    match va {
        serde_json::Value::Number(a) => match vb {
            serde_json::Value::Number(b) => a.as_f64().unwrap() == b.as_f64().unwrap(),
            _ => false,
        },
        _ => *va == *vb,
    }
}
