use either::Either;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct Schema {
    /// [JSON reference](https://tools.ietf.org/html/draft-pbryan-zyp-json-ref-03)
    /// path to another definition
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "$ref")]
    pub ref_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "$defs")]
    pub defs: Option<IndexMap<String, Schema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

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
    pub items: Option<Box<Schema>>,
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
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "additionalProperties"
    )]
    pub pattern_properties: Option<IndexMap<String, Schema>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "additionalProperties"
    )]
    pub additional_properties: Option<BoolOrSchema>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enum")]
    pub enum_value: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "const")]
    pub const_value: Option<serde_json::Value>,

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "default")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "readOnly")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "writeOnly")]
    pub write_only: Option<bool>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<Map<String, Value>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct BoolOrSchema {
    #[serde(with = "either::serde_untagged")]
    pub inner: Either<bool, Box<Schema>>,
}

impl Default for BoolOrSchema {
    fn default() -> Self {
        Self {
            inner: Either::Left(false),
        }
    }
}
