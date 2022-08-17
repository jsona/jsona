use either::Either;
use fancy_regex::Regex;
use indexmap::IndexMap;
use jsona::dom::{KeyOrIndex, Keys, Node};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::HashSet, fmt::Display};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "$ref")]
    pub ref_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "$defs")]
    pub defs: Option<IndexMap<String, Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub schema_type: Option<OneOrMultiTypes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "default")]
    pub default: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(rename = "exclusiveMaximum", skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<bool>,
    #[serde(rename = "exclusiveMinimum", skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<bool>,
    #[serde(rename = "multipleOf", skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,

    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<OneOrMultiSchemas>,
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u32>,
    #[serde(rename = "uniqueItems", skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,
    #[serde(rename = "additionalItems", skip_serializing_if = "Option::is_none")]
    pub additional_items: Option<BoolOrSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<Box<Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, Schema>>,
    #[serde(rename = "maxProperties", skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<u32>,
    #[serde(rename = "minProperties", skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "patternProperties")]
    pub pattern_properties: Option<IndexMap<String, Schema>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "additionalProperties"
    )]
    pub additional_properties: Option<BoolOrSchema>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enum")]
    pub enum_value: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "const")]
    pub const_value: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "readOnly")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "writeOnly")]
    pub write_only: Option<bool>,

    #[serde(rename = "allOf", skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<Schema>>,
    #[serde(rename = "oneOf", skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<Schema>>,
    #[serde(rename = "anyOf", skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(rename = "not", skip_serializing_if = "Option::is_none")]
    pub not: Option<Vec<Schema>>,
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub if_value: Option<Box<Schema>>,
    #[serde(rename = "then", skip_serializing_if = "Option::is_none")]
    pub then_value: Option<Box<Schema>>,
    #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_value: Option<Box<Schema>>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<Map<String, Value>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct BoolOrSchema {
    #[serde(with = "either::serde_untagged")]
    pub value: Either<bool, Box<Schema>>,
}

impl Default for BoolOrSchema {
    fn default() -> Self {
        Self {
            value: Either::Left(false),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    String,
    Number,
    Boolean,
    Integer,
    Null,
    Object,
    Array,
}

impl SchemaType {
    pub fn from_node(node: &Node) -> Option<Self> {
        let schema_type = match &node {
            Node::Null(v) => {
                if v.is_valid() {
                    SchemaType::Null
                } else {
                    return None;
                }
            }
            Node::Bool(_) => SchemaType::Boolean,
            Node::Number(v) => {
                if v.is_integer() {
                    SchemaType::Integer
                } else {
                    SchemaType::Number
                }
            }
            Node::String(_) => SchemaType::String,
            Node::Array(_) => SchemaType::Array,
            Node::Object(_) => SchemaType::Object,
        };
        Some(schema_type)
    }
}

impl Display for SchemaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_str = match self {
            SchemaType::String => "string",
            SchemaType::Number => "number",
            SchemaType::Integer => "integer",
            SchemaType::Boolean => "boolean",
            SchemaType::Null => "null",
            SchemaType::Object => "object",
            SchemaType::Array => "array",
        };
        f.write_str(type_str)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct OneOrMultiTypes {
    #[serde(with = "either::serde_untagged")]
    value: Either<SchemaType, Vec<SchemaType>>,
}

impl OneOrMultiTypes {
    pub fn new(items: impl Iterator<Item = SchemaType>) -> Self {
        let mut items: Vec<SchemaType> = items.collect();
        if items.len() > 1 {
            Self {
                value: Either::Right(items),
            }
        } else {
            Self {
                value: Either::Left(items.remove(0)),
            }
        }
    }
    pub fn contains(&self, target: &SchemaType) -> bool {
        match self.value.as_ref() {
            Either::Left(value) => value == target,
            Either::Right(values) => values.iter().any(|v| v == target),
        }
    }
    pub fn types(&self) -> HashSet<SchemaType> {
        match self.value.as_ref() {
            Either::Left(value) => [value.clone()].into(),
            Either::Right(values) => values.iter().cloned().collect(),
        }
    }
    pub fn len(&self) -> usize {
        match self.value.as_ref() {
            Either::Left(_) => 1,
            Either::Right(values) => values.len(),
        }
    }
}

impl From<SchemaType> for OneOrMultiTypes {
    fn from(schema_type: SchemaType) -> Self {
        Self {
            value: Either::Left(schema_type),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct OneOrMultiSchemas {
    #[serde(with = "either::serde_untagged")]
    value: Either<Box<Schema>, Vec<Schema>>,
}

impl OneOrMultiSchemas {
    pub fn new(mut items: Vec<Schema>) -> Self {
        if items.len() > 1 {
            Self {
                value: Either::Right(items),
            }
        } else {
            Self {
                value: Either::Left(Box::new(items.remove(0))),
            }
        }
    }
    pub fn as_ref_vec(&self) -> Vec<&Schema> {
        match self.value.as_ref() {
            Either::Left(schema) => vec![schema],
            Either::Right(schemas) => schemas.iter().collect(),
        }
    }
}

impl Schema {
    pub fn pointer(&self, keys: &Keys) -> Vec<&Schema> {
        let mut result = vec![];
        pointer_impl(&mut result, self, self, keys);
        result
    }
    pub fn maybe_type(&self, schema_type: &SchemaType) -> bool {
        self.schema_type
            .as_ref()
            .map(|v| v.contains(schema_type))
            .unwrap_or_default()
    }
    pub fn one_type(&self) -> Option<SchemaType> {
        self.schema_type
            .as_ref()
            .and_then(|v| v.value.as_ref().left())
            .cloned()
    }
    pub fn types(&self) -> HashSet<SchemaType> {
        self.schema_type
            .as_ref()
            .map(|v| v.types())
            .unwrap_or_default()
    }
    pub fn debug_string(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

fn pointer_impl<'a>(
    result: &mut Vec<&'a Schema>,
    root_schema: &'a Schema,
    local_schema: &'a Schema,
    keys: &Keys,
) {
    let local_schema = match resolve(root_schema, local_schema) {
        Some(v) => v,
        None => return,
    };
    if let Some(schemas) = local_schema
        .one_of
        .as_ref()
        .or(local_schema.any_of.as_ref())
        .or(local_schema.all_of.as_ref())
    {
        for schema in schemas.iter() {
            pointer_impl(result, root_schema, schema, keys);
        }
    } else {
        match keys.shift() {
            None => {
                result.push(local_schema);
            }
            Some((key, keys)) => match key {
                KeyOrIndex::Index(index) => {
                    if let Some(local_schema) = local_schema.items.as_ref() {
                        match local_schema.value.as_ref() {
                            Either::Left(local_schema) => {
                                pointer_impl(result, root_schema, local_schema, &keys)
                            }
                            Either::Right(schemas) => {
                                if let Some(local_schema) = schemas.get(index) {
                                    pointer_impl(result, root_schema, local_schema, &keys)
                                }
                            }
                        }
                    }
                }
                KeyOrIndex::Key(key) => {
                    if let Some(local_schema) = local_schema
                        .properties
                        .as_ref()
                        .and_then(|v| v.get(key.value()))
                    {
                        pointer_impl(result, root_schema, local_schema, &keys)
                    }
                    if let Some(schemas) = local_schema.pattern_properties.as_ref() {
                        for (pat, local_schema) in schemas.iter() {
                            if let Ok(re) = Regex::new(pat) {
                                if let Ok(true) = re.is_match(key.value()) {
                                    pointer_impl(result, root_schema, local_schema, &keys)
                                }
                            }
                        }
                    }
                    if let Some(local_schema) = local_schema
                        .additional_properties
                        .as_ref()
                        .and_then(|v| v.value.as_ref().right())
                    {
                        pointer_impl(result, root_schema, local_schema, &keys)
                    }
                }
                _ => {}
            },
        }
    }
}

fn resolve<'a>(root_schema: &'a Schema, local_schema: &'a Schema) -> Option<&'a Schema> {
    let schema = match local_schema.ref_value.as_ref() {
        Some(ref_value) => {
            if ref_value == "#" {
                root_schema
            } else {
                match root_schema.defs.as_ref().and_then(|defs| {
                    Regex::new(r#"^#/\$defs/(\w+)$"#)
                        .ok()
                        .and_then(|v| v.captures(ref_value).ok().flatten().and_then(|v| v.get(1)))
                        .and_then(|v| defs.get(v.as_str()))
                }) {
                    Some(v) => v,
                    None => return None,
                }
            }
        }
        None => local_schema,
    };
    Some(schema)
}
