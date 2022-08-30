use either::Either;
use fancy_regex::Regex;
use indexmap::IndexMap;
use jsona::dom::{KeyOrIndex, Keys, Node, ParseError};
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::{cell::RefCell, rc::Rc, str::FromStr};
use std::{collections::HashSet, fmt::Display};
use thiserror::Error;

pub type SchemaResult<T> = std::result::Result<T, SchemaError>;

pub const REF_PREFIX: &str = "#/$defs/";

pub static REF_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^#/\$defs/(\w+)$"#).unwrap());

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid jsona")]
    InvalidJsona(#[from] ParseError),
    #[error("invalid schema")]
    InvalidSchema(#[from] SchemaError),
}

#[derive(Clone, Debug, Error)]
pub enum SchemaError {
    #[error("conflict def {name}")]
    ConflictDef { keys: Keys, name: String },
    #[error("unknown ref {name}")]
    UnknownRef { keys: Keys, name: String },
    #[error("the type is unexpected")]
    UnexpectedType { keys: Keys },
    #[error("the schema type is not match value type")]
    UnmatchedSchemaType { keys: Keys },
    #[error("invalid schema value, {error}")]
    InvalidSchemaValue { keys: Keys, error: String },
    #[error("invalid compound value")]
    InvalidCompoundValue { keys: Keys },
}

impl SchemaError {
    pub fn keys(&self) -> &Keys {
        match self {
            SchemaError::ConflictDef { keys, .. } => keys,
            SchemaError::UnknownRef { keys, .. } => keys,
            SchemaError::UnexpectedType { keys } => keys,
            SchemaError::UnmatchedSchemaType { keys } => keys,
            SchemaError::InvalidSchemaValue { keys, .. } => keys,
            SchemaError::InvalidCompoundValue { keys } => keys,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct Schema {
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub ref_value: Option<String>,
    #[serde(rename = "$defs", skip_serializing_if = "Option::is_none")]
    pub defs: Option<IndexMap<String, Schema>>,
    #[serde(rename = "$id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "$comment", skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<OneOrMultiTypes>,

    #[serde(rename = "default", skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<Number>,
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
    #[serde(rename = "contentEncoding", skip_serializing_if = "Option::is_none")]
    pub content_encoding: Option<String>,
    #[serde(rename = "contentMediaType", skip_serializing_if = "Option::is_none")]
    pub content_media_type: Option<String>,

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
    #[serde(rename = "maxContains", skip_serializing_if = "Option::is_none")]
    pub max_contains: Option<u32>,
    #[serde(rename = "minContains", skip_serializing_if = "Option::is_none")]
    pub min_contains: Option<u32>,
    #[serde(rename = "unevaluatedItems", skip_serializing_if = "Option::is_none")]
    pub unevaluated_items: Option<BoolOrSchema>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, Schema>>,
    #[serde(rename = "maxProperties", skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<u32>,
    #[serde(rename = "minProperties", skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(rename = "patternProperties", skip_serializing_if = "Option::is_none")]
    pub pattern_properties: Option<IndexMap<String, Schema>>,
    #[serde(
        rename = "additionalProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_properties: Option<BoolOrSchema>,
    #[serde(rename = "dependentRequired", skip_serializing_if = "Option::is_none")]
    pub dependent_required: Option<IndexMap<String, Vec<String>>>,
    #[serde(rename = "dependentSchemas", skip_serializing_if = "Option::is_none")]
    pub dependent_schemas: Option<IndexMap<String, Schema>>,
    #[serde(rename = "propertyNames", skip_serializing_if = "Option::is_none")]
    pub property_names: Option<Box<Schema>>,
    #[serde(
        rename = "unevaluatedProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub unevaluated_properties: Option<BoolOrSchema>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "enum")]
    pub enum_value: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "const")]
    pub const_value: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<Value>>,
    #[serde(rename = "readOnly", skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(rename = "writeOnly", skip_serializing_if = "Option::is_none")]
    pub write_only: Option<bool>,

    #[serde(rename = "allOf", skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<Schema>>,
    #[serde(rename = "oneOf", skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<Schema>>,
    #[serde(rename = "anyOf", skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(rename = "not", skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<Schema>>,
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub if_value: Option<Box<Schema>>,
    #[serde(rename = "then", skip_serializing_if = "Option::is_none")]
    pub then_value: Option<Box<Schema>>,
    #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_value: Option<Box<Schema>>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<Map<String, Value>>,
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

impl TryFrom<&Node> for Schema {
    type Error = SchemaError;

    fn try_from(node: &Node) -> SchemaResult<Self> {
        let scope = SchemaParser {
            keys: Keys::default(),
            node: node.clone(),
            defs: Default::default(),
            ref_prefix: Rc::new(REF_PREFIX.to_string()),
            prefer_optional: true,
        };
        let mut schema = scope.parse()?;
        let defs = scope.defs.take();
        if !defs.is_empty() {
            schema.defs = Some(defs);
        }
        Ok(schema)
    }
}

impl FromStr for Schema {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let node: Node = s.parse()?;
        let schema = Schema::try_from(&node)?;
        Ok(schema)
    }
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

    pub fn match_node(&self, node: &Node) -> bool {
        match self {
            SchemaType::String => node.is_string(),
            SchemaType::Number => node.is_number(),
            SchemaType::Boolean => node.is_bool(),
            SchemaType::Integer => node.is_integer(),
            SchemaType::Null => node.is_null(),
            SchemaType::Object => node.is_object(),
            SchemaType::Array => node.is_array(),
        }
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
    pub value: Either<SchemaType, Vec<SchemaType>>,
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
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    pub value: Either<Box<Schema>, Vec<Schema>>,
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
}
#[derive(Debug, Clone)]
pub struct SchemaParser {
    pub node: Node,
    pub keys: Keys,
    pub defs: Rc<RefCell<IndexMap<String, Schema>>>,
    pub ref_prefix: Rc<String>,
    pub prefer_optional: bool,
}

impl SchemaParser {
    pub fn parse(&self) -> SchemaResult<Schema> {
        let mut def_value = String::new();
        if let Some(def) = self.parse_string_annotation("@def")? {
            let mut defs = self.defs.borrow_mut();
            if defs.contains_key(&def) {
                return Err(SchemaError::ConflictDef {
                    keys: self.keys.clone(),
                    name: def,
                });
            }
            defs.insert(def.clone(), Default::default());
            def_value = def;
        } else if let Some(ref_value) = self.parse_string_annotation("@ref")? {
            let defs = self.defs.borrow();
            if !defs.contains_key(&ref_value) {
                return Err(SchemaError::UnknownRef {
                    keys: self.keys.clone(),
                    name: ref_value,
                });
            }
            return Ok(Schema {
                ref_value: Some(format!("{}{}", self.ref_prefix, ref_value)),
                ..Default::default()
            });
        }
        let mut schema: Schema = self.parse_object_annotation("@schema")?.unwrap_or_default();
        if let Some(describe) = self.parse_string_annotation("@describe")? {
            schema.description = Some(describe);
        }
        if self.exist_annotation("@example") {
            schema.examples = Some(vec![self.node.to_plain_json()])
        }
        if self.exist_annotation("@default") {
            schema.default = Some(self.node.to_plain_json())
        }
        let schema_types = schema.types();
        let node_type = SchemaType::from_node(&self.node);
        if schema_types.is_empty() {
            schema.schema_type = node_type.map(Into::into);
        } else if let Some(node_type) = node_type {
            if !schema_types.contains(&node_type) {
                return Err(SchemaError::UnmatchedSchemaType {
                    keys: self.keys.clone(),
                });
            }
        }
        match &self.node {
            Node::Object(obj) => {
                for (key, child) in obj.value().read().iter() {
                    let child_parser = self.spawn(key.clone(), child.clone());
                    let key = key.value();
                    let pattern = child_parser.parse_string_annotation("@pattern")?;
                    let child_schema = child_parser.parse()?;
                    if let Some(pattern) = pattern {
                        let props = schema.pattern_properties.get_or_insert(Default::default());
                        props.insert(pattern, child_schema);
                    } else {
                        let props = schema.properties.get_or_insert(Default::default());
                        props.insert(key.to_string(), child_schema);
                        if (self.prefer_optional && child_parser.exist_annotation("@required"))
                            || (!self.prefer_optional
                                && !child_parser.exist_annotation("@optional"))
                        {
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
                        let child_parser = self.spawn(i, child.clone());
                        schemas.push(child_parser.parse()?);
                    }
                    if let Some(compound) = self.parse_string_annotation("@compound")? {
                        schema.schema_type = None;
                        match compound.as_str() {
                            "anyOf" => schema.any_of = Some(schemas),
                            "oneOf" => schema.one_of = Some(schemas),
                            "allOf" => schema.all_of = Some(schemas),
                            _ => {
                                return Err(SchemaError::InvalidCompoundValue {
                                    keys: self.keys.join(KeyOrIndex::annotation("@compound")),
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
        if self.exist_annotation("@anytype") {
            schema.schema_type = None;
        }
        if !def_value.is_empty() {
            self.defs.borrow_mut().insert(def_value.clone(), schema);
            return Ok(Schema {
                ref_value: Some(format!("{}{}", self.ref_prefix, def_value)),
                ..Default::default()
            });
        }
        Ok(schema)
    }

    fn spawn(&self, key: impl Into<KeyOrIndex>, node: Node) -> Self {
        Self {
            node,
            keys: self.keys.clone().join(key.into()),
            defs: self.defs.clone(),
            ref_prefix: self.ref_prefix.clone(),
            prefer_optional: self.prefer_optional,
        }
    }

    fn exist_annotation(&self, name: &str) -> bool {
        self.node.get(&KeyOrIndex::annotation(name)).is_some()
    }

    fn parse_object_annotation<T: DeserializeOwned>(&self, name: &str) -> SchemaResult<Option<T>> {
        match self.node.get_as_object(name) {
            Some((key, Some(value))) => {
                let value = Node::from(value).to_plain_json();
                match serde_json::from_value(value) {
                    Ok(v) => Ok(Some(v)),
                    Err(err) => Err(SchemaError::InvalidSchemaValue {
                        keys: self.keys.clone().join(key),
                        error: err.to_string(),
                    }),
                }
            }
            Some((key, None)) => Err(SchemaError::UnexpectedType {
                keys: self.keys.clone().join(key),
            }),
            None => Ok(None),
        }
    }

    fn parse_string_annotation(&self, name: &str) -> SchemaResult<Option<String>> {
        match self.node.get_as_string(name) {
            Some((_, Some(value))) => Ok(Some(value.value().to_string())),
            Some((key, None)) => Err(SchemaError::UnexpectedType {
                keys: self.keys.clone().join(key),
            }),
            None => Ok(None),
        }
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
                    REF_REGEX
                        .captures(ref_value)
                        .ok()
                        .flatten()
                        .and_then(|v| v.get(1))
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
