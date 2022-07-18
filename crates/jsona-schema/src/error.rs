use jsona::dom::{Error as DomError, Keys};
use jsona::parser::Error as ParserError;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid syntax")]
    Syntax(Vec<ParserError>),
    #[error("invalid dom node")]
    Dom(Vec<DomError>),
    #[error("invalid value at {keys}")]
    InvalidValue { keys: Keys },
    #[error("invalid type at {keys}")]
    MismatchType { keys: Keys },
    #[error("conflict at {keys}")]
    Conflict { keys: Keys },
    #[error("conflict def {0}")]
    ConflictDef(String),
    #[error("missed def {0}")]
    MissedDef(String),
}
