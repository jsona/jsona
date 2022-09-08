mod validates;

use std::convert::TryFrom;

use indexmap::IndexMap;
use jsona::dom::{visit_annotations, Key, KeyOrIndex, Keys, Node};
use jsona_schema::{SchemaError, SchemaResult, SchemaType};
pub use validates::Error as JSONASchemaValidationError;

pub use jsona_schema::Schema;

pub const VALUE_KEY: &str = "_";

#[derive(Debug)]
pub struct JSONASchemaValidator {
    schema: Schema,
    annotations: Schema,
}

impl JSONASchemaValidator {
    pub fn validate(&self, node: &Node) -> Vec<JSONASchemaValidationError> {
        let mut collect_errors = vec![];
        let default_defs = Default::default();
        let defs = self.schema.defs.as_ref().unwrap_or(&default_defs);
        if let Some(value_schema) = self.get_entry_schema(VALUE_KEY) {
            collect_errors.extend(validates::validate(
                defs,
                value_schema,
                &Keys::default(),
                node,
            ));
        }
        for (keys, value) in visit_annotations(node).into_iter() {
            if let Some(key) = keys.last_annotation_key().and_then(|v| v.annotation_name()) {
                if let Some(schema) = self.get_entry_schema(&key) {
                    collect_errors.extend(validates::validate(defs, schema, &keys, &value));
                }
            }
        }
        collect_errors
    }

    pub fn pointer(&self, keys: &Keys) -> Vec<&Schema> {
        let (annotation_key, keys) = keys.shift_annotation();
        let new_keys = match annotation_key {
            Some(key) => match key.annotation_name() {
                Some(name) => {
                    if !self.contains_annotation_key(&name) {
                        return vec![&self.annotations];
                    }
                    let new_keys = Keys::new(
                        [KeyOrIndex::property(&name), KeyOrIndex::property("value")].into_iter(),
                    );
                    new_keys.extend(keys)
                }
                None => {
                    return vec![&self.annotations];
                }
            },
            None => {
                let new_keys = Keys::new(
                    [
                        KeyOrIndex::property(VALUE_KEY),
                        KeyOrIndex::property("value"),
                    ]
                    .into_iter(),
                );
                new_keys.extend(keys)
            }
        };
        self.schema.pointer(&new_keys)
    }

    pub fn get_entry_schema(&self, key: &str) -> Option<&Schema> {
        let properties = self.schema.properties.as_ref()?;
        let schema = properties.get(key)?;
        let properties = schema.properties.as_ref()?;
        properties.get("value")
    }

    pub fn contains_annotation_key(&self, key: &str) -> bool {
        match self.annotations.properties.as_ref() {
            Some(properties) => properties.contains_key(key),
            None => false,
        }
    }
}

impl TryFrom<&Node> for JSONASchemaValidator {
    type Error = Vec<SchemaError>;
    fn try_from(value: &Node) -> Result<Self, Self::Error> {
        let object = match value.as_object() {
            Some(v) => v,
            None => {
                return Err(vec![SchemaError::InvalidSchemaValue {
                    keys: Keys::default(),
                    error: "must be object".into(),
                }])
            }
        };
        let mut annotation_schemas = IndexMap::default();
        let mut errors = vec![];
        for (key, value) in object.value().read().iter() {
            let keys = Keys::single(key.clone());
            if key.is_quote() {
                errors.push(SchemaError::InvalidSchemaValue {
                    keys,
                    error: "invalid name".into(),
                });
                continue;
            }
            let object = match value.as_object() {
                Some(v) => v,
                None => {
                    errors.push(SchemaError::InvalidSchemaValue {
                        keys,
                        error: "must be object".into(),
                    });
                    continue;
                }
            };
            let value_key = Key::property("value");
            let keys = keys.join(value_key.clone());
            match object.get(&value_key) {
                Some(value) => {
                    if key.value() != VALUE_KEY {
                        let schema = Schema {
                            description: parse_string_annotation(&keys, &value, "@describe")
                                .ok()
                                .flatten(),
                            schema_type: SchemaType::from_node(&value).map(|v| v.into()),
                            ..Default::default()
                        };
                        annotation_schemas.insert(key.value().to_string(), schema);
                    }
                }
                None => {
                    errors.push(SchemaError::InvalidSchemaValue {
                        keys,
                        error: "must be object".into(),
                    });
                    continue;
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let schema = Schema::try_from(value)?;
        let annotations = Schema {
            schema_type: Some(SchemaType::Object.into()),
            properties: if annotation_schemas.is_empty() {
                None
            } else {
                Some(annotation_schemas)
            },
            ..Default::default()
        };
        Ok(JSONASchemaValidator {
            schema,
            annotations,
        })
    }
}

fn parse_string_annotation(keys: &Keys, node: &Node, name: &str) -> SchemaResult<Option<String>> {
    match node.get_as_string(name) {
        Some((_, Some(value))) => Ok(Some(value.value().to_string())),
        Some((key, None)) => Err(vec![SchemaError::UnexpectedType {
            keys: keys.clone().join(key),
        }]),
        None => Ok(None),
    }
}
