use either::Either;
use fancy_regex::Regex;
use indexmap::IndexMap;
use jsona::dom::{KeyOrIndex, Keys};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
    pub schema_type: Option<String>,
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
    pub items: Option<SchemaOrSchemaArray>,
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

    #[serde(rename = "not", skip_serializing_if = "Option::is_none")]
    pub not: Option<Vec<Schema>>,
    #[serde(rename = "allOf", skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<Schema>>,
    #[serde(rename = "oneOf", skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<Schema>>,
    #[serde(rename = "anyOf", skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub if_value: Option<BoolOrSchema>,
    #[serde(rename = "then", skip_serializing_if = "Option::is_none")]
    pub then_value: Option<BoolOrSchema>,
    #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_value: Option<BoolOrSchema>,

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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct SchemaOrSchemaArray {
    #[serde(with = "either::serde_untagged")]
    pub value: Either<Box<Schema>, Vec<Schema>>,
}

impl SchemaOrSchemaArray {
    pub fn to_vec(&self) -> Vec<&Schema> {
        match self.value.as_ref() {
            Either::Left(schema) => vec![schema],
            Either::Right(schemas) => schemas.iter().collect(),
        }
    }
}

impl Schema {
    pub fn pointer(&self, keys: &Keys) -> Vec<&Schema> {
        let mut result = vec![];
        let mut pointed = false;
        pointer_impl(&mut result, self, self, keys, &mut pointed);
        result
    }
    pub fn is_object(&self) -> bool {
        self.schema_type
            .as_ref()
            .map(|v| v == "object")
            .unwrap_or_default()
    }
    pub fn is_array(&self) -> bool {
        self.schema_type
            .as_ref()
            .map(|v| v == "array")
            .unwrap_or_default()
    }
    pub fn is_string(&self) -> bool {
        self.schema_type
            .as_ref()
            .map(|v| v == "string")
            .unwrap_or_default()
    }
    pub fn is_number(&self) -> bool {
        self.schema_type
            .as_ref()
            .map(|v| v == "number")
            .unwrap_or_default()
    }
    pub fn is_null(&self) -> bool {
        self.schema_type
            .as_ref()
            .map(|v| v == "null")
            .unwrap_or_default()
    }
    pub fn is_boolean(&self) -> bool {
        self.schema_type
            .as_ref()
            .map(|v| v == "boolean")
            .unwrap_or_default()
    }
}

fn pointer_impl<'a>(
    result: &mut Vec<&'a Schema>,
    root_schema: &'a Schema,
    local_schema: &'a Schema,
    keys: &Keys,
    pointed: &mut bool,
) {
    let local_schema = match local_schema.ref_value.as_ref() {
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
                    None => return,
                }
            }
        }
        None => local_schema,
    };
    if let Some(schemas) = local_schema
        .one_of
        .as_ref()
        .or(local_schema.any_of.as_ref())
        .or(local_schema.all_of.as_ref())
    {
        let mut pointed = false;
        for schema in schemas.iter() {
            pointer_impl(result, root_schema, schema, keys, &mut pointed);
            if pointed && local_schema.one_of.is_some() {
                break;
            }
        }
    } else {
        let (key, keys) = keys.shift();
        match key {
            None => {
                result.push(local_schema);
                *pointed = true;
            }
            Some(key) => match key {
                KeyOrIndex::Index(index) => {
                    if let Some(local_schema) = local_schema.items.as_ref() {
                        match local_schema.value.as_ref() {
                            Either::Left(local_schema) => {
                                pointer_impl(result, root_schema, local_schema, &keys, pointed)
                            }
                            Either::Right(schemas) => {
                                if let Some(local_schema) = schemas.get(index) {
                                    pointer_impl(result, root_schema, local_schema, &keys, pointed)
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
                        pointer_impl(result, root_schema, local_schema, &keys, pointed)
                    }
                    if let Some(schemas) = local_schema.pattern_properties.as_ref() {
                        for (pat, local_schema) in schemas.iter() {
                            if let Ok(re) = Regex::new(pat) {
                                if re.is_match(key.value()).is_ok() {
                                    pointer_impl(result, root_schema, local_schema, &keys, pointed)
                                }
                            }
                        }
                    }
                    if let Some(local_schema) = local_schema
                        .additional_properties
                        .as_ref()
                        .and_then(|v| v.value.as_ref().right())
                    {
                        pointer_impl(result, root_schema, local_schema, &keys, pointed)
                    }
                }
                _ => {}
            },
        }
    }
}
