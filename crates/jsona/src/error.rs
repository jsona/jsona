use crate::dom::DomError;
use crate::util::mapper::{Mapper, Range};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid syntax")]
    InvalidSyntax { errors: Vec<crate::parser::Error> },
    #[error("invalid dom")]
    InvalidDom { errors: Vec<DomError> },
}

impl Error {
    pub fn to_error_objects(&self, mapper: &Mapper) -> Vec<ErrorObject> {
        match self {
            Error::InvalidSyntax { errors } => errors
                .iter()
                .map(|err| {
                    let message = err.to_string();
                    let range = mapper.range(err.range);
                    ErrorObject::new("InvalidSyntax", message, range)
                })
                .collect(),
            Error::InvalidDom { errors } => errors
                .iter()
                .flat_map(|err| {
                    let message = err.to_string();
                    match err {
                        DomError::ConflictingKeys { key, other_key } => {
                            let key_range = key.mapper_range(mapper);
                            let other_key_range = other_key.mapper_range(mapper);
                            vec![
                                ErrorObject::new("ConflictingKeys", message.clone(), key_range),
                                ErrorObject::new("ConflictingKeys", message, other_key_range),
                            ]
                        }
                        DomError::InvalidNode { syntax } => {
                            let range = mapper.range(syntax.text_range());
                            vec![ErrorObject::new("InvalidNode", message, range)]
                        }
                        DomError::InvalidNumber { syntax } => {
                            let range = mapper.range(syntax.text_range());
                            vec![ErrorObject::new("InvalidNumber", message, range)]
                        }
                        DomError::InvalidString { syntax } => {
                            let range = mapper.range(syntax.text_range());
                            vec![ErrorObject::new("InvalidString", message, range)]
                        }
                    }
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ErrorObject {
    pub kind: String,
    pub message: String,
    pub range: Option<Range>,
}

impl ErrorObject {
    pub fn new(kind: &str, message: String, range: Option<Range>) -> Self {
        Self {
            kind: kind.into(),
            message,
            range,
        }
    }
}
