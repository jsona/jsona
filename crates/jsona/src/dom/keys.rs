use crate::parser::Parser;
use crate::util::mapper;

use super::from_syntax::keys_from_syntax;
use super::node::Key;
use super::Node;

use rowan::TextRange;
use std::iter::{empty, once};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOrIndex {
    Index(usize),
    Key(Key),
}

impl From<usize> for KeyOrIndex {
    fn from(v: usize) -> Self {
        Self::Index(v)
    }
}

impl From<Key> for KeyOrIndex {
    fn from(k: Key) -> Self {
        Self::Key(k)
    }
}

impl core::fmt::Display for KeyOrIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyOrIndex::Index(v) => write!(f, "[{}]", v),
            KeyOrIndex::Key(v) => {
                if v.is_property() {
                    write!(f, ".{}", v)
                } else {
                    write!(f, "{}", v)
                }
            }
        }
    }
}

impl KeyOrIndex {
    pub fn property<T: Into<String>>(key: T) -> Self {
        Self::Key(Key::property(key))
    }

    pub fn annotation<T: Into<String>>(key: T) -> Self {
        Self::Key(Key::annotation(key))
    }

    pub fn is_index(&self) -> bool {
        matches!(self, KeyOrIndex::Index(_))
    }

    pub fn is_key(&self) -> bool {
        matches!(self, KeyOrIndex::Key(_))
    }

    pub fn is_property_key(&self) -> bool {
        if let KeyOrIndex::Key(v) = self {
            if v.is_property() {
                return true;
            }
        }
        false
    }

    pub fn is_annotation_key(&self) -> bool {
        if let KeyOrIndex::Key(v) = self {
            if v.is_annotation() {
                return true;
            }
        }
        false
    }

    pub fn as_index(&self) -> Option<&usize> {
        if let KeyOrIndex::Index(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_key(&self) -> Option<&Key> {
        if let KeyOrIndex::Key(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_property_key(&self) -> Option<&Key> {
        if let KeyOrIndex::Key(v) = self {
            if v.is_property() {
                return Some(v);
            }
        }
        None
    }

    pub fn as_annotation_key(&self) -> Option<&Key> {
        if let KeyOrIndex::Key(v) = self {
            if v.is_annotation() {
                return Some(v);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Keys {
    dotted: Arc<str>,
    keys: Arc<[KeyOrIndex]>,
}

impl Keys {
    pub fn new(keys: impl Iterator<Item = KeyOrIndex>) -> Self {
        let keys: Arc<[KeyOrIndex]> = keys.collect();
        let mut dotted = String::new();
        for k in keys.iter() {
            dotted.push_str(&k.to_string());
        }
        let dotted: Arc<str> = Arc::from(dotted);
        Self { keys, dotted }
    }

    pub fn single(key: impl Into<KeyOrIndex>) -> Self {
        Self::new(once(key.into()))
    }

    pub fn join(&self, key: impl Into<KeyOrIndex>) -> Self {
        self.extend(once(key.into()))
    }

    pub fn extend<I, K>(&self, keys: I) -> Self
    where
        I: IntoIterator<Item = K>,
        K: Into<KeyOrIndex>,
    {
        Self::new(
            self.keys
                .iter()
                .cloned()
                .chain(keys.into_iter().map(Into::into)),
        )
    }

    pub fn first(&self) -> Option<&KeyOrIndex> {
        self.keys.first()
    }

    pub fn last(&self) -> Option<&KeyOrIndex> {
        self.keys.last()
    }

    pub fn last_property_key(&self) -> Option<&Key> {
        self.last().and_then(|v| v.as_property_key())
    }

    pub fn last_annotation_key(&self) -> Option<&Key> {
        self.last().and_then(|v| v.as_annotation_key())
    }

    pub fn last_text_range(&self) -> Option<TextRange> {
        match self.last() {
            Some(KeyOrIndex::Key(k)) => k.syntax().map(|v| v.text_range()),
            _ => None,
        }
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &KeyOrIndex> + DoubleEndedIterator {
        self.keys.iter()
    }

    pub fn iter_keys(&self) -> Vec<Keys> {
        (0..self.keys.len() + 1)
            .map(|v| Keys::new(self.keys.iter().take(v).cloned()))
            .collect()
    }

    pub fn dotted(&self) -> &str {
        &self.dotted
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.len() == 0
    }

    pub fn parent(&self) -> Option<Keys> {
        if self.len() < 2 {
            return None;
        }
        Some(Keys::new(self.iter().take(self.len() - 1).cloned()))
    }

    pub fn shift(&self) -> Option<(KeyOrIndex, Self)> {
        if self.is_empty() {
            return None;
        }
        let (left, right) = self.keys.split_at(1);
        Some((
            left.get(0).cloned().unwrap(),
            Self::new(right.iter().cloned()),
        ))
    }

    pub fn shift_annotation(&self) -> (Option<Key>, Self) {
        match self.keys.iter().enumerate().find(|(_, k)| {
            if let KeyOrIndex::Key(k) = k {
                k.is_annotation()
            } else {
                false
            }
        }) {
            Some((i, k)) => (
                Some(k.as_annotation_key().cloned().unwrap()),
                Self::new(self.keys.iter().skip(i + 1).cloned()),
            ),
            None => (None, self.clone()),
        }
    }

    pub fn mapper_range(&self, node: &Node, mapper: &mapper::Mapper) -> Option<mapper::Range> {
        let key = self.last().and_then(|v| v.as_key())?;
        let key_range = key.mapper_range(mapper)?;
        match node.path(self).and_then(|v| v.mapper_range(mapper)) {
            Some(value_range) => Some(key_range.join(&value_range)),
            None => Some(key_range),
        }
    }
}

impl Default for Keys {
    fn default() -> Self {
        Self::new(empty())
    }
}

impl IntoIterator for Keys {
    type Item = KeyOrIndex;

    type IntoIter = std::vec::IntoIter<KeyOrIndex>;

    fn into_iter(self) -> Self::IntoIter {
        Vec::from(&*self.keys).into_iter()
    }
}

impl core::fmt::Display for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.dotted.fmt(f)
    }
}

impl FromStr for Keys {
    type Err = Vec<crate::parser::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Parser::new(s).parse_keys_only(true);
        if !p.errors.is_empty() {
            return Err(p.errors);
        }
        Ok(Keys::new(keys_from_syntax(&p.into_syntax().into())))
    }
}

impl PartialEq for Keys {
    fn eq(&self, other: &Self) -> bool {
        self.dotted == other.dotted
    }
}

impl Eq for Keys {}

impl std::hash::Hash for Keys {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.dotted.hash(state);
    }
}
