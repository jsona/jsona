use super::node::Key;
use crate::syntax::SyntaxElement;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("the syntax was not expected")]
    UnexpectedSyntax { syntax: SyntaxElement },
    #[error("the string contains invalid escape sequence(s)")]
    InvalidEscapeSequence { syntax: SyntaxElement },
    #[error("the syntax was not valid number")]
    InvalidNumber { syntax: SyntaxElement },
    #[error("conflicting keys")]
    ConflictingKeys { key: Key, other: Key },
}

#[derive(Debug, Clone, Error)]
pub enum QueryError {
    #[error("the key or index was not found")]
    NotFound,
    #[error("mismatch value type")]
    MismatchType,
    #[error("the given key is invalid: {0}")]
    InvalidKey(crate::parser::Error),
}

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("invalid syntax")]
    InvalidSyntax { errors: Vec<crate::parser::Error> },
    #[error("invalid dom")]
    InvalidDom { errors: Vec<Error> },
}
