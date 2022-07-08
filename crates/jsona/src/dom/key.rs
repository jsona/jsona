use super::error::Error;
use crate::syntax::{SyntaxElement, SyntaxKind};
use crate::util::escape::unescape;
use crate::util::shared::Shared;

use logos::Lexer;
use once_cell::unsync::OnceCell;
use rowan::NodeOrToken;
use std::fmt::Write;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOrIndex {
    Index(usize),
    Key(Key),
    AnnoKey(Key),
}

impl<N> From<N> for KeyOrIndex
where
    N: Into<usize>,
{
    fn from(v: N) -> Self {
        Self::Index(v.into())
    }
}

impl core::fmt::Display for KeyOrIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyOrIndex::Index(v) => v.fmt(f),
            KeyOrIndex::Key(v) => v.fmt(f),
            KeyOrIndex::AnnoKey(v) => write!(f, "@{}", v),
        }
    }
}

impl KeyOrIndex {
    pub fn new_key(k: Key) -> Self {
        Self::Key(k)
    }

    pub fn new_anno_key(k: Key) -> Self {
        Self::AnnoKey(k)
    }

    /// Returns `true` if the key or index is [`Index`].
    ///
    /// [`Index`]: KeyOrIndex::Index
    pub fn is_index(&self) -> bool {
        matches!(self, Self::Index(..))
    }

    /// Returns `true` if the key or index is [`Key`].
    ///
    /// [`Key`]: KeyOrIndex::Key
    pub fn is_key(&self) -> bool {
        matches!(self, Self::Key(..))
    }

    /// Returns `true` if the key or index is [`AnnoKey`].
    ///
    /// [`Key`]: KeyOrIndex::Key
    pub fn is_anno_key(&self) -> bool {
        matches!(self, Self::AnnoKey(..))
    }

    pub fn as_index(&self) -> Option<&usize> {
        if let Self::Index(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_key(&self) -> Option<&Key> {
        if let Self::Key(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_anno_key(&self) -> Option<&Key> {
        if let Self::AnnoKey(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub(crate) struct KeyInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) is_valid: bool,
    pub(crate) value: OnceCell<String>,
}

#[derive(Debug, Clone)]
pub struct Key {
    inner: Arc<KeyInner>,
}

impl KeyInner {
    #[allow(dead_code)]
    pub(crate) fn wrap(self) -> Key {
        self.into()
    }
}

impl From<KeyInner> for Key {
    fn from(inner: KeyInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl<S> From<S> for Key
where
    S: Into<String>,
{
    fn from(s: S) -> Self {
        Key::new(s)
    }
}

impl Key {
    /// Return a new key with the given value.
    ///
    /// # Remarks
    ///
    /// This **does not** check or modify the input string.
    pub fn new(key: impl Into<String>) -> Self {
        KeyInner {
            errors: Default::default(),
            syntax: None,
            is_valid: true,
            value: OnceCell::from(key.into()),
        }
        .wrap()
    }

    /// An unescaped value of the key.
    pub fn value(&self) -> &str {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .and_then(NodeOrToken::as_token)
                .map(|s| {
                    if s.text().starts_with('\'') {
                        let string = s.text();
                        let string = string.strip_prefix('\'').unwrap_or(string);
                        let string = string.strip_suffix('\'').unwrap_or(string);
                        string.to_string()
                    } else if s.text().starts_with('"') {
                        let string = s.text();
                        let string = string.strip_prefix('"').unwrap_or(string);
                        let string = string.strip_suffix('"').unwrap_or(string);
                        match unescape(string) {
                            Ok(s) => s,
                            Err(_) => {
                                self.inner.errors.update(|errors| {
                                    errors.push(Error::InvalidEscapeSequence {
                                        string: s.clone().into(),
                                    })
                                });
                                String::new()
                            }
                        }
                    } else {
                        s.text().to_string()
                    }
                })
                .unwrap_or_default()
        })
    }
}

impl AsRef<str> for Key {
    fn as_ref(&self) -> &str {
        self.value()
    }
}

impl core::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = &self.inner.syntax {
            return s.fmt(f);
        }

        if !matches!(
            Lexer::<SyntaxKind>::new(self.value()).next(),
            Some(SyntaxKind::IDENT) | None
        ) {
            f.write_char('\'')?;
            self.value().fmt(f)?;
            f.write_char('\'')?;
            return Ok(());
        }

        self.value().fmt(f)
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        if !self.inner.is_valid || !other.inner.is_valid {
            return false;
        }

        self.value().eq(other.value())
    }
}

impl Eq for Key {}

impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if !self.inner.is_valid {
            return 0.hash(state);
        }

        self.value().hash(state)
    }
}
