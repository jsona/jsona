use anyhow::{anyhow, bail};
use indexmap::IndexMap;
use jsona::{
    dom::{KeyOrIndex, Keys, Node},
    parser,
};
use jsona_schema::{from_node, Schema};
use jsonschema::{error::ValidationErrorKind, paths::JSONPointer, JSONSchema};
use serde::{Deserialize, Serialize};

pub struct JSONASchema {
    value: JSONSchema,
    annotations: IndexMap<String, JSONSchema>,
}

impl JSONASchema {
    pub fn new(schema: &JSONASchemaValue) -> Result<Self, anyhow::Error> {
        let value = compile_json_schema(&schema.value)
            .map_err(|err| anyhow!("invalid value schema, {}", err))?;
        let mut annotations = IndexMap::default();
        for (key, value) in &schema.annotations {
            let annotation = compile_json_schema(value)
                .map_err(|err| anyhow!("invalid @{} schema, {}", key, err))?;
            annotations.insert(key.to_string(), annotation);
        }
        Ok(JSONASchema { value, annotations })
    }
    pub fn validate(&self, node: &Node) -> Result<Vec<NodeValidationError>, anyhow::Error> {
        let mut errors = vec![];
        jsona_schema_validate(&self.value, &mut errors, node, Keys::default())?;
        for (keys, annotation_node) in node.annotation_iter() {
            if let Some(schema) = keys
                .last()
                .and_then(|v| v.as_annotation_key())
                .and_then(|v| self.annotations.get(&v.to_string()))
            {
                jsona_schema_validate(schema, &mut errors, &annotation_node, keys)?;
            }
        }
        Ok(errors)
    }
}

fn compile_json_schema(schema: &Schema) -> Result<JSONSchema, anyhow::Error> {
    let json = serde_json::to_value(schema)?;
    JSONSchema::options()
        .compile(&json)
        .map_err(|e| anyhow!("{}", e))
}

fn jsona_schema_validate(
    schema: &JSONSchema,
    errors: &mut Vec<NodeValidationError>,
    node: &Node,
    keys: Keys,
) -> Result<(), anyhow::Error> {
    let value = node.to_plain_json();
    if let Err(errs) = schema.validate(&value) {
        for err in errs {
            let info = err.to_string();
            let error =
                NodeValidationError::new(node, keys.clone(), err.kind, err.instance_path, info)?;
            errors.push(error);
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct JSONASchemaValue {
    value: Box<Schema>,
    annotations: IndexMap<String, Schema>,
}

impl JSONASchemaValue {
    pub fn from_slice(data: &[u8]) -> Result<Self, anyhow::Error> {
        let data = std::str::from_utf8(data)?;
        let parse = parser::parse(data);
        if !parse.errors.is_empty() {
            bail!("invalid jsona");
        }
        let node = parse.into_dom();
        Self::from_node(node)
    }
    pub fn from_node(node: Node) -> Result<Self, anyhow::Error> {
        if node.validate().is_err() {
            bail!("invalid jsona");
        };
        let value_node = node
            .try_get(&KeyOrIndex::property("value"))
            .map_err(|_| anyhow!("failed to get value at .value"))?;
        let value_schema =
            from_node(&value_node).map_err(|_| anyhow!("failed to parse schema at .value"))?;
        let mut annotations: IndexMap<String, Schema> = Default::default();
        let annotations_value = node
            .try_get_as_object(&KeyOrIndex::property("annotations"))
            .map_err(|_| anyhow!("failed to parse annotations"))?;
        if let Some(annotations_value) = annotations_value {
            for (key, value) in annotations_value.value().read().iter() {
                let schmea = from_node(value)
                    .map_err(|_| anyhow!("failed to parse schema at .annotations.{}", key))?;
                annotations.insert(key.value().to_string(), schmea);
            }
        }
        Ok(JSONASchemaValue {
            value: value_schema.into(),
            annotations,
        })
    }
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
    ) -> Result<Self, anyhow::Error> {
        let mut keys = keys;
        let mut node = node.clone();

        'outer: for path in &instance_path {
            match path {
                jsonschema::paths::PathChunk::Property(p) => match node {
                    Node::Object(t) => {
                        let entries = t.value().read();
                        for (k, entry) in entries.iter() {
                            if k.value() == &**p {
                                keys = keys.join(k.into());
                                node = entry.clone();
                                continue 'outer;
                            }
                        }
                        return Err(anyhow!("invalid key {} at {}", p, keys));
                    }
                    _ => return Err(anyhow!("invalid key {} at {}", p, keys)),
                },
                jsonschema::paths::PathChunk::Index(idx) => {
                    node = node
                        .try_get(&KeyOrIndex::Index(*idx))
                        .map_err(|_| anyhow!("invalid index {} at {}", idx, keys))?;
                    keys = keys.join((*idx).into());
                }
                jsonschema::paths::PathChunk::Keyword(_) => {}
            }
        }

        Ok(Self {
            keys,
            node,
            kind,
            info,
        })
    }
}
