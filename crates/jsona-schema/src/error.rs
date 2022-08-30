use jsona::{
    dom::{Keys, Node},
    error::ErrorObject,
    util::mapper::Mapper,
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
    pub fn to_error_objects(&self, node: &Node, mapper: &Mapper) -> Vec<ErrorObject> {
        let message = self.to_string();
        let (kind, range) = match self {
            SchemaError::ConflictDef { keys, .. } => {
                ("ConflictDef", keys.mapper_range(node, mapper))
            }
            SchemaError::UnknownRef { keys, .. } => ("UnknownRef", keys.mapper_range(node, mapper)),
            SchemaError::UnexpectedType { keys } => {
                ("UnexpectedType", keys.mapper_range(node, mapper))
            }
            SchemaError::UnmatchedSchemaType { keys } => {
                ("UnmatchedSchemaType", keys.mapper_range(node, mapper))
            }
            SchemaError::InvalidSchemaValue { keys, .. } => {
                ("InvalidSchemaValue", keys.mapper_range(node, mapper))
            }
            SchemaError::InvalidCompoundValue { keys } => {
                ("InvalidCompoundValue", keys.mapper_range(node, mapper))
            }
        };
        vec![ErrorObject::new(kind, message, range)]
    }
}
