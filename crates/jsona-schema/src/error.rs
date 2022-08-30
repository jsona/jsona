use jsona::{
    dom::{Keys, Node},
    error::ErrorObject,
    util::mapper::{Mapper, Range},
};
use thiserror::Error;

pub type SchemaResult<T> = std::result::Result<T, SchemaError>;

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
    pub fn into_error_objects(&self, node: &Node, mapper: &Mapper) -> Vec<ErrorObject> {
        let message = self.to_string();
        let (kind, range) = match self {
            SchemaError::ConflictDef { keys, .. } => ("ConflictDef", get_range(keys, node, mapper)),
            SchemaError::UnknownRef { keys, .. } => ("UnknownRef", get_range(keys, node, mapper)),
            SchemaError::UnexpectedType { keys } => {
                ("UnexpectedType", get_range(keys, node, mapper))
            }
            SchemaError::UnmatchedSchemaType { keys } => {
                ("UnmatchedSchemaType", get_range(keys, node, mapper))
            }
            SchemaError::InvalidSchemaValue { keys, .. } => {
                ("InvalidSchemaValue", get_range(keys, node, mapper))
            }
            SchemaError::InvalidCompoundValue { keys } => {
                ("InvalidCompoundValue", get_range(keys, node, mapper))
            }
        };
        vec![ErrorObject::new(kind, message, range)]
    }
}

fn get_range(keys: &Keys, node: &Node, mapper: &Mapper) -> Option<Range> {
    let key = keys.last().and_then(|v| v.as_key())?;
    let key_range = key.mapper_range(mapper)?;
    match node.path(keys).and_then(|v| v.mapper_range(mapper)) {
        Some(value_range) => Some(key_range.join(&value_range)),
        None => Some(key_range),
    }
}
