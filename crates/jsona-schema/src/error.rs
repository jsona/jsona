use jsona::{
    dom::{Keys, Node},
    error::ErrorObject,
    util::mapper::Mapper,
};
use thiserror::Error;

pub type SchemaResult<T> = std::result::Result<T, Vec<SchemaError>>;

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

    pub fn append_keys(self, prefix_keys: &Keys) -> Self {
        match self {
            SchemaError::ConflictDef { keys, name } => SchemaError::ConflictDef {
                keys: prefix_keys.extend(keys),
                name,
            },
            SchemaError::UnknownRef { keys, name } => SchemaError::UnknownRef {
                keys: prefix_keys.extend(keys),
                name,
            },
            SchemaError::UnexpectedType { keys } => SchemaError::UnexpectedType {
                keys: prefix_keys.extend(keys),
            },
            SchemaError::UnmatchedSchemaType { keys } => SchemaError::UnmatchedSchemaType {
                keys: prefix_keys.extend(keys),
            },
            SchemaError::InvalidSchemaValue { keys, error } => SchemaError::InvalidSchemaValue {
                keys: prefix_keys.extend(keys),
                error,
            },
            SchemaError::InvalidCompoundValue { keys } => SchemaError::UnmatchedSchemaType {
                keys: prefix_keys.extend(keys),
            },
        }
    }

    pub fn to_error_object(&self, node: &Node, mapper: &Mapper) -> ErrorObject {
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
        ErrorObject::new(kind, message, range)
    }
}
