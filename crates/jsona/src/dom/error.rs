use super::node::Key;
use crate::syntax::SyntaxElement;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("the syntax is not valid node")]
    InvalidNode { syntax: SyntaxElement },
    #[error("the syntax is not valid string")]
    InvalidString { syntax: SyntaxElement },
    #[error("the syntax is not valid number")]
    InvalidNumber { syntax: SyntaxElement },
    #[error("conflicting keys")]
    ConflictingKeys { key: Key, other: Key },
}

#[derive(Debug, Clone, Error)]
pub enum KeyError {
    #[error("the value is not found")]
    NotFound,
    #[error("the glob is unexpected: {0}")]
    UnexpectedGlob(String),
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
