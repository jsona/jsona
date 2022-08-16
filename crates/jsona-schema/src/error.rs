use jsona::dom::Keys;
use std::fmt;

/// An error that can occur during validation.
#[derive(Clone, Debug)]
pub struct Error {
    pub keys: Keys,
    /// Type of validation error.
    pub kind: ErrorKind,
}

/// Kinds of errors that may happen during validation
#[derive(Clone, Debug)]
pub enum ErrorKind {
    InvalidFile,
    InvalidValue,
    MismatchType,
    ConflictPattern(String),
    ConflictDef(String),
    UnknownDef(String),
}

impl Error {
    pub const fn invalid_file(keys: Keys) -> Self {
        Error {
            keys,
            kind: ErrorKind::InvalidFile,
        }
    }
    pub const fn invalid_value(keys: Keys) -> Self {
        Error {
            keys,
            kind: ErrorKind::InvalidValue,
        }
    }
    pub const fn mismatch_type(keys: Keys) -> Self {
        Error {
            keys,
            kind: ErrorKind::MismatchType,
        }
    }
    pub fn conflict_pattern(keys: Keys, pattern: &str) -> Self {
        Error {
            keys,
            kind: ErrorKind::ConflictPattern(pattern.to_string()),
        }
    }
    pub fn conflict_def(keys: Keys, def: &str) -> Self {
        Error {
            keys,
            kind: ErrorKind::ConflictDef(def.to_string()),
        }
    }
    pub fn unknown_def(keys: Keys, def: &str) -> Self {
        Error {
            keys,
            kind: ErrorKind::UnknownDef(def.to_string()),
        }
    }
}

impl std::error::Error for Error {}

/// Textual representation of various validation errors.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidFile => f.write_str("invalid jsona file"),
            ErrorKind::InvalidValue => f.write_str("invalid value"),
            ErrorKind::MismatchType => f.write_str("mismatch type"),
            ErrorKind::ConflictPattern(name) => write!(f, "conflict pattern {}", name),
            ErrorKind::ConflictDef(name) => write!(f, "conflict def {}", name),
            ErrorKind::UnknownDef(name) => write!(f, "unknown def {}", name),
        }
    }
}
