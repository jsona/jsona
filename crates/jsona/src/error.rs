use crate::dom::DomError;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid syntax")]
    InvalidSyntax { errors: Vec<crate::parser::Error> },
    #[error("invalid dom")]
    InvalidDom { errors: Vec<DomError> },
}
