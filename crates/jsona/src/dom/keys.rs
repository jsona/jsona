use super::error::{Error, QueryError};
use super::from_syntax::keys_from_syntax;
use super::node::Key;
use crate::parser::Parser;
use crate::util::text_range::join_ranges;

use rowan::TextRange;
use std::iter::{empty, once};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOrIndex {
    Index(usize),
    Key(Key),
    AnnotationKey(Key),
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
            KeyOrIndex::AnnotationKey(v) => v.fmt(f),
        }
    }
}

impl KeyOrIndex {
    pub fn new_key(k: Key) -> Self {
        Self::Key(k)
    }

    pub fn new_anno_key(k: Key) -> Self {
        Self::AnnotationKey(k)
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
    pub fn is_annotation_key(&self) -> bool {
        matches!(self, Self::AnnotationKey(..))
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

    pub fn as_annotation_key(&self) -> Option<&Key> {
        if let Self::AnnotationKey(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn to_join_string(&self) -> String {
        match self {
            KeyOrIndex::Index(_) => format!(".{}", self),
            KeyOrIndex::Key(_) => format!(".{}", self),
            KeyOrIndex::AnnotationKey(_) => format!("@{}", self),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Keys {
    dotted: Arc<str>,
    keys: Arc<[KeyOrIndex]>,
}

impl Keys {
    #[inline]
    pub fn empty() -> Self {
        Self::new(empty())
    }

    pub fn single(key: impl Into<KeyOrIndex>) -> Self {
        Self::new(once(key.into()))
    }

    pub fn new(keys: impl Iterator<Item = KeyOrIndex>) -> Self {
        let keys: Arc<[KeyOrIndex]> = keys.collect();
        let mut dotted = String::new();
        for (i, k) in keys.iter().enumerate() {
            if i == 0 {
                dotted.push_str(&k.to_string());
            } else {
                dotted.push_str(&k.to_join_string());
            }
        }
        let dotted: Arc<str> = Arc::from(dotted);
        Self { keys, dotted }
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

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &KeyOrIndex> + DoubleEndedIterator {
        self.keys.iter()
    }

    pub fn iter_keys(&self) -> Vec<Keys> {
        (0..self.keys.len() + 1)
            .into_iter()
            .map(|v| Keys::new(self.keys.iter().take(v).cloned()))
            .collect()
    }

    pub fn dotted(&self) -> &str {
        &*self.dotted
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.len() == 0
    }

    pub fn common_prefix_count(&self, other: &Self) -> usize {
        self.iter()
            .zip(other.iter())
            .take_while(|(a, b)| a == b)
            .count()
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.len() >= other.len() && self.common_prefix_count(other) == other.len()
    }

    pub fn part_of(&self, other: &Self) -> bool {
        other.contains(self)
    }

    pub fn skip_left(&self, n: usize) -> Self {
        Self::new(self.keys.iter().skip(n).cloned())
    }

    pub fn skip_right(&self, n: usize) -> Self {
        Self::new(self.keys.iter().rev().skip(n).cloned().rev())
    }

    pub fn all_text_range(&self) -> TextRange {
        join_ranges(self.keys.iter().filter_map(|key| match key {
            KeyOrIndex::Index(_) => None,
            KeyOrIndex::Key(k) => k.text_range(),
            KeyOrIndex::AnnotationKey(k) => k.text_range(),
        }))
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
        self.dotted().fmt(f)
    }
}

impl FromStr for Keys {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut p = Parser::new(s).parse_keys_only();
        if let Some(err) = p.errors.pop() {
            return Err(QueryError::InvalidKey(err).into());
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

impl<N> From<N> for Keys
where
    N: Into<usize>,
{
    fn from(v: N) -> Self {
        Keys::new(once(v.into().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keys() {
        let keys = [
            "object.array.1.foo",
            "object.array[1].foo",
            "object.array[*].foo",
            "object.array.*.foo",
            "dependencies.tokio-*.version",
            "object.array.1.foo@bar",
            "*@foo",
            "**@foo",
        ];
        for v in keys {
            let keys: Result<Keys, _> = v.parse();
            assert!(keys.is_ok());
        }
    }
}
