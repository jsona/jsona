use super::node::Key;
use crate::syntax::SyntaxElement;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DomError {
    #[error("the syntax is not valid node")]
    InvalidNode { syntax: SyntaxElement },
    #[error("the syntax is not valid string")]
    InvalidString { syntax: SyntaxElement },
    #[error("the syntax is not valid number")]
    InvalidNumber { syntax: SyntaxElement },
    #[error("conflicting keys")]
    ConflictingKeys { key: Key, other: Key },
}
