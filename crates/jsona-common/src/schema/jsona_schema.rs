use jsona::dom::{Keys, Node};
use jsona::value::Value;
use std::fmt::{self, Formatter};

pub struct JsonaSchema {}

impl JsonaSchema {
    pub fn compile(_value: &Value) -> Result<Self, anyhow::Error> {
        todo!()
    }
    pub fn validate(&self, _node: &Node) -> Result<(), Vec<ValidationError>> {
        todo!()
    }
}
/// An error that can occur during validation.
#[derive(Debug)]
pub struct ValidationError {
    pub keys: Keys,
    pub node: Node,
    /// Type of validation error.
    pub kind: ValidationErrorKind,
}
/// Textual representation of various validation errors.
impl fmt::Display for ValidationError {
    #[allow(clippy::too_many_lines)] // The function is long but it does formatting only
    #[inline]
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Kinds of errors that may happen during validation
#[derive(Debug)]
#[allow(missing_docs)]
pub enum ValidationErrorKind {}
