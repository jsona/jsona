use super::error::{Error, QueryError};
use super::from_syntax::keys_from_syntax;
use super::node::Key;
use super::KeyMatchKind;
use crate::parser::Parser;
use crate::util::pattern::Pattern;
use crate::util::text_range::join_ranges;

use rowan::TextRange;
use std::iter::{empty, once};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOrIndex {
    Index(usize),
    ValueKey(Key),
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
            KeyOrIndex::ValueKey(v) => v.fmt(f),
            KeyOrIndex::AnnotationKey(v) => write!(f, "@{}", v),
        }
    }
}

impl KeyOrIndex {
    pub fn new_value_key(k: Key) -> Self {
        Self::ValueKey(k)
    }

    pub fn new_annotation_key(k: Key) -> Self {
        Self::AnnotationKey(k)
    }

    /// Returns `true` if the key or index is [`Index`].
    ///
    /// [`Index`]: KeyOrIndex::Index
    pub fn is_index(&self) -> bool {
        matches!(self, Self::Index(..))
    }

    /// Returns `true` if the key or index is [`ValueKey`].
    ///
    /// [`ValueKey`]: KeyOrIndex::ValueKey
    pub fn is_value_key(&self) -> bool {
        matches!(self, Self::ValueKey(..))
    }

    /// Returns `true` if the key or index is [`AnnotationKey`].
    ///
    /// [`AnnotationKey`]: KeyOrIndex::AnnotationKey
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

    pub fn as_value_key(&self) -> Option<&Key> {
        if let Self::ValueKey(v) = self {
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
            KeyOrIndex::ValueKey(_) => format!(".{}", self),
            KeyOrIndex::AnnotationKey(_) => format!("{}", self),
        }
    }

    pub fn match_kind(&self) -> KeyMatchKind {
        match self {
            KeyOrIndex::Index(_) => KeyMatchKind::Normal,
            KeyOrIndex::ValueKey(v) => v.match_kind(),
            KeyOrIndex::AnnotationKey(v) => v.match_kind(),
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
            KeyOrIndex::ValueKey(k) => k.text_range(),
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
        if s.is_empty() {
            return Ok(Keys::new(vec![].into_iter()));
        }
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

#[derive(Debug)]
pub struct KeysMatcher {
    keys1: Pattern,
    keys2: Pattern,
    match_kind: KeyMatchKind,
    match_children: bool,
    keys: Keys,
    value: String,
}

impl KeysMatcher {
    pub fn new(keys: &Keys, match_children: bool) -> Result<Self, Error> {
        let mut keys1 = String::new();
        let mut keys2 = String::new();
        let mut match_kind = KeyMatchKind::Normal;
        let mut at = false;
        for k in keys.iter() {
            if !at && k.is_annotation_key() {
                at = true;
            }
            let keys = match at {
                true => &mut keys2,
                false => &mut keys1,
            };
            if keys.is_empty() {
                keys.push_str(&k.to_string());
            } else {
                keys.push_str(&k.to_join_string());
            }
            match_kind = match_kind.max(k.match_kind())
        }
        Ok(Self {
            value: format!("{}{}", keys1, keys2),
            keys1: Pattern::new(&keys1).map_err(QueryError::InvalidGlob)?,
            keys2: Pattern::new(&keys2).map_err(QueryError::InvalidGlob)?,
            match_kind,
            match_children,
            keys: keys.clone(),
        })
    }
    pub fn is_match(&self, keys: &Keys) -> bool {
        match (self.match_kind, self.match_children) {
            (KeyMatchKind::Normal, true) => {
                if self.keys.len() > keys.len() {
                    return false;
                }
                keys.to_string().starts_with(&self.value)
            }
            (KeyMatchKind::Normal, false) => {
                if self.keys.len() != keys.len() {
                    return false;
                }
                keys.to_string() == self.value
            }
            (KeyMatchKind::MatchOne, true) => {
                if self.keys.len() > keys.len() {
                    return false;
                }
                let (k1, k2) = Self::split(keys);
                self.keys1.matches(&k1) && self.keys2.matches(&k2)
            }
            (KeyMatchKind::MatchOne, false) => {
                let (k1, k2) = Self::split(keys);
                if self.keys.len() != keys.len() {
                    return false;
                }
                self.keys1.matches(&k1) && self.keys2.matches(&k2)
            }
            (KeyMatchKind::MatchMulti, _) => {
                if self.keys.len() > keys.len() {
                    return false;
                }
                let (k1, k2) = Self::split(keys);
                self.keys1.matches(&k1) && self.keys2.matches(&k2)
            }
        }
    }

    fn split(keys: &Keys) -> (String, String) {
        let mut keys1 = String::new();
        let mut keys2 = String::new();
        let mut at = false;
        for k in keys.iter() {
            if !at && k.is_annotation_key() {
                at = true;
            }
            let keys = match at {
                true => &mut keys2,
                false => &mut keys1,
            };
            if keys.is_empty() {
                keys.push_str(&k.to_string());
            } else {
                keys.push_str(&k.to_join_string());
            }
        }
        (keys1, keys2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keys() {
        assert!("object.array.1.foo".parse::<Keys>().is_ok());
        assert!("object.array.1.foo".parse::<Keys>().is_ok());
        assert!("object.array[1].foo".parse::<Keys>().is_ok());
        assert!("object.array[*].foo".parse::<Keys>().is_ok());
        assert!("object.array.*.foo".parse::<Keys>().is_ok());
        assert!("dependencies.tokio*.version".parse::<Keys>().is_ok());
        assert!("object.array.1.foo@bar".parse::<Keys>().is_ok());
        assert!(r#"object."a-b""#.parse::<Keys>().is_ok());
        assert!("*@foo".parse::<Keys>().is_ok());
        assert!("**@foo".parse::<Keys>().is_ok());
        assert!("".parse::<Keys>().is_ok());
    }

    #[test]
    fn test_parse_keys_fails() {
        assert!("object..1".parse::<Keys>().is_err());
        assert!("object.a-b".parse::<Keys>().is_err());
        assert!("object.a-*".parse::<Keys>().is_err());
        assert!("object.a**".parse::<Keys>().is_err());
        assert!("*@foo@bar".parse::<Keys>().is_err());
    }

    #[test]
    fn test_keys_to_string() {
        assert_eq!(
            "object.array[1].foo".parse::<Keys>().unwrap().to_string(),
            "object.array.1.foo"
        );
        assert_eq!(
            "object.array.'1'.foo".parse::<Keys>().unwrap().to_string(),
            "object.array.'1'.foo"
        );
        assert_eq!(
            "object.array.'a-b'.foo"
                .parse::<Keys>()
                .unwrap()
                .to_string(),
            "object.array.'a-b'.foo"
        );
        assert_eq!(
            "dependencies.tokio*.version"
                .parse::<Keys>()
                .unwrap()
                .to_string(),
            "dependencies.tokio*.version"
        );
        assert_eq!(
            r#"object."a-b""#.parse::<Keys>().unwrap().to_string(),
            r#"object."a-b""#
        );
    }
}
