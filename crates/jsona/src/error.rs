use crate::util::mapper::Range;
use crate::{dom::DomError, util::mapper};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid syntax")]
    InvalidSyntax { errors: Vec<crate::parser::Error> },
    #[error("invalid dom")]
    InvalidDom { errors: Vec<DomError> },
}

trait IntoErrorObjects {
    fn into_error_objects(self, mapper: &mapper::Mapper) -> Vec<ErrorObject>;
}

impl IntoErrorObjects for Error {
    fn into_error_objects(self, mapper: &mapper::Mapper) -> Vec<ErrorObject> {
        let mut error_objects: Vec<ErrorObject> = vec![];
        match self {
            Error::InvalidSyntax { errors } => {
                for err in errors.into_iter() {
                    let message = err.to_string();
                    let range = mapper.range(err.range);
                    error_objects.push(ErrorObject::new("InvalidSyntax", &message, range));
                }
            }
            Error::InvalidDom { errors } => {
                for err in errors.into_iter() {
                    let message = err.to_string();
                    match err {
                        DomError::ConflictingKeys { key, other_key } => {
                            let range = key.mapper_range(mapper);
                            error_objects.push(ErrorObject::new(
                                "ConflictingKeys",
                                &message,
                                range,
                            ));
                            let range = other_key.mapper_range(mapper);
                            error_objects.push(ErrorObject::new(
                                "ConflictingKeys",
                                &message,
                                range,
                            ));
                        }
                        DomError::InvalidNode { syntax } => {
                            let range = mapper.range(syntax.text_range());
                            error_objects.push(ErrorObject::new("InvalidNode", &message, range));
                        }
                        DomError::InvalidNumber { syntax } => {
                            let range = mapper.range(syntax.text_range());
                            error_objects.push(ErrorObject::new("InvalidNumber", &message, range));
                        }
                        DomError::InvalidString { syntax } => {
                            let range = mapper.range(syntax.text_range());
                            error_objects.push(ErrorObject::new("InvalidString", &message, range));
                        }
                    }
                }
            }
        }
        error_objects
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ErrorObject {
    pub kind: String,
    pub message: String,
    pub range: Option<Range>,
}

impl ErrorObject {
    pub fn new(kind: &str, message: &str, range: Option<Range>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            range,
        }
    }
}
