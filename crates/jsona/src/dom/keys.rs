use super::error::QueryError;
use super::from_syntax::keys_from_syntax;
use super::node::Key;
use super::DomNode;
use crate::parser::Parser;
use crate::util::glob;

use rowan::TextRange;
use std::iter::{empty, once};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOrIndex {
    Index(usize),
    Key(Key),
    GlobIndex(String),
    GlobKey(String),
    AnyRecursive,
}

impl<'a> From<&'a usize> for KeyOrIndex {
    fn from(v: &'a usize) -> Self {
        Self::Index(*v)
    }
}

impl<'a> From<&'a Key> for KeyOrIndex {
    fn from(k: &'a Key) -> Self {
        Self::Key(k.clone())
    }
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
            KeyOrIndex::GlobIndex(v) => write!(f, "[{}]", v),
            KeyOrIndex::GlobKey(v) => write!(f, ".{}", v),
            KeyOrIndex::AnyRecursive => write!(f, "**"),
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

    pub fn is_match(&self, other: &Self) -> bool {
        match self {
            KeyOrIndex::Index(_) | KeyOrIndex::Key(_) => self == other,
            KeyOrIndex::GlobIndex(k) => match other {
                KeyOrIndex::Index(v) => glob(k, &v.to_string()),
                _ => false,
            },
            KeyOrIndex::GlobKey(k) => match other {
                // NOTE: glob key only works on property key
                KeyOrIndex::Key(v) => v.is_property() && glob(k, v.value()),
                _ => false,
            },
            KeyOrIndex::AnyRecursive => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Keys {
    dotted: Arc<str>,
    keys: Arc<[KeyOrIndex]>,
    all_plain: bool,
    exist_any_recursive: bool,
}

impl Keys {
    pub fn single(key: impl Into<KeyOrIndex>) -> Self {
        Self::new(once(key.into()))
    }

    pub fn new(keys: impl Iterator<Item = KeyOrIndex>) -> Self {
        let keys: Arc<[KeyOrIndex]> = keys.collect();
        let mut dotted = String::new();
        let mut all_plain = true;
        let mut exist_any_recursive = false;
        for k in keys.iter() {
            match k {
                KeyOrIndex::Index(_) | KeyOrIndex::Key(_) => {}
                KeyOrIndex::GlobIndex(_) | KeyOrIndex::GlobKey(_) => {
                    all_plain = false;
                }
                KeyOrIndex::AnyRecursive => {
                    all_plain = false;
                    exist_any_recursive = true;
                }
            }
            dotted.push_str(&k.to_string());
        }
        let dotted: Arc<str> = Arc::from(dotted);
        Self {
            keys,
            dotted,
            all_plain,
            exist_any_recursive,
        }
    }

    pub fn join(&self, key: KeyOrIndex) -> Self {
        self.extend(once(key))
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
    pub fn is_plain(&self) -> bool {
        self.all_plain
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

    pub fn is_match(&self, other: &Keys, match_children: bool) -> bool {
        if !self.exist_any_recursive {
            if self.len() > other.len() || !match_children && self.len() != other.len() {
                false
            } else {
                self.iter()
                    .zip(other.iter())
                    .all(|(v1, v2)| v1.is_match(v2))
            }
        } else {
            let keys: Vec<&KeyOrIndex> = self.iter().collect();
            let target_keys: Vec<&KeyOrIndex> = other.iter().collect();
            let mut i = 0;
            let mut j = 0;
            'outer: while i < self.len() {
                let key = keys[i];
                match key {
                    KeyOrIndex::Index(_)
                    | KeyOrIndex::Key(_)
                    | KeyOrIndex::GlobIndex(_)
                    | KeyOrIndex::GlobKey(_) => match target_keys.get(j) {
                        Some(target_key) => {
                            if key.is_match(target_key) {
                                j += 1;
                                i += 1;
                                continue;
                            } else {
                                return false;
                            }
                        }
                        _ => return false,
                    },
                    KeyOrIndex::AnyRecursive => {
                        if let Some(key) = keys.get(i + 1) {
                            let mut matched_target = false;
                            while let Some(target_key) = target_keys.get(j) {
                                if key.is_match(target_key) {
                                    matched_target = true;
                                } else if matched_target {
                                    j -= 1;
                                    i += 2;
                                    continue 'outer;
                                }
                                j += 1;
                            }
                            if matched_target {
                                i += 2;
                                continue 'outer;
                            } else {
                                return false;
                            }
                        } else {
                            return true;
                        }
                    }
                }
            }
            if match_children {
                true
            } else {
                j >= target_keys.len() - 1
            }
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
        self.dotted().fmt(f)
    }
}

impl FromStr for Keys {
    type Err = QueryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s == "." {
            return Ok(Keys::default());
        }
        let mut p = Parser::new(s).parse_keys_only();
        if let Some(err) = p.errors.pop() {
            return Err(QueryError::InvalidKey(err));
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_parse_keys {
        ($v1:literal) => {
            assert_eq!($v1.parse::<Keys>().unwrap().to_string(), $v1);
        };
        ($v1:literal, $v2:literal) => {
            assert_eq!($v1.parse::<Keys>().unwrap().to_string(), $v2);
        };
    }

    macro_rules! assert_match_keys {
        ($v1:literal, $v2:literal) => {
            assert_eq!(
                $v1.parse::<Keys>()
                    .unwrap()
                    .is_match(&$v2.parse::<Keys>().unwrap(), false),
                true
            );
        };
        ($v1:literal, $v2:literal, $v3:literal) => {
            assert_eq!(
                $v1.parse::<Keys>()
                    .unwrap()
                    .is_match(&$v2.parse::<Keys>().unwrap(), false),
                $v3
            );
        };
        ($v1:literal, $v2:literal, $v3:literal, $v4:literal) => {
            assert_eq!(
                $v1.parse::<Keys>()
                    .unwrap()
                    .is_match(&$v2.parse::<Keys>().unwrap(), $v4),
                $v3
            );
        };
    }

    #[test]
    fn test_parse_keys() {
        assert_parse_keys!("");
        assert_parse_keys!("[1]");
        assert_parse_keys!("[*]");
        assert_parse_keys!(".foo");
        assert_parse_keys!("foo", ".foo");
        assert_parse_keys!(".*");
        assert_parse_keys!("**");
        assert_parse_keys!("**.**", "**");
        assert_parse_keys!(".foo.bar");
        assert_parse_keys!(".foo@bar");
        assert_parse_keys!("@foo");
        assert_parse_keys!("[0].foo");
        assert_parse_keys!("[0][1]");
        assert_parse_keys!("[*].foo");
        assert_parse_keys!("[*][1]");
        assert_parse_keys!("[*]@foo");
        assert_parse_keys!(".*@foo");
        assert_parse_keys!(".foo*");
        assert_parse_keys!(".foo.*");
        assert_parse_keys!(".foo.*.bar");
        assert_parse_keys!(r#".foo."ba-z""#, r#".foo."ba-z""#);
        assert_parse_keys!(r#".foo."ba z""#, r#".foo."ba z""#);
        assert_parse_keys!(".foo.1");
        assert_parse_keys!(".foo.1.baz");
        assert_parse_keys!(r#".foo."1".baz"#, ".foo.1.baz");
        assert_parse_keys!("*foo", ".*foo");
        assert_parse_keys!("**@foo");
        assert_parse_keys!("**.*");
    }

    #[test]
    fn test_parse_keys_fails() {
        assert!("..foo".parse::<Keys>().is_err());
        assert!("foo.b-z".parse::<Keys>().is_err());
        assert!("foo.".parse::<Keys>().is_err());
        assert!("foo.b-*".parse::<Keys>().is_err());
        assert!("foo.b**".parse::<Keys>().is_err());
    }

    #[test]
    fn test_match_keys() {
        assert_match_keys!("**", ".foo");
        assert_match_keys!("**", "[1]");
        assert_match_keys!("**", ".foo.bar");
        assert_match_keys!(".*", ".foo");
        assert_match_keys!(".*", ".foo.bar", false);
        assert_match_keys!("**.a?c", ".abc");
        assert_match_keys!("**.a?c", ".foo.abc");
        assert_match_keys!("**.*", ".foo");
        assert_match_keys!("**.*", ".foo.bar");
        assert_match_keys!("**.*", "[1]", false);
        assert_match_keys!("**[*]", "[1]");
        assert_match_keys!("**[*]", ".foo", false);
        assert_match_keys!(".abc", ".abc");
        assert_match_keys!(".a*c", ".abc");
        assert_match_keys!(".a*c", ".abbc");
        assert_match_keys!(".a?c", ".abc");
        assert_match_keys!(".a?c", ".abdc", false);
        assert_match_keys!(".abc@foo", ".abc@foo");
        assert_match_keys!("@foo", "@foo");
        assert_match_keys!("**@foo", ".a.b@foo");
        assert_match_keys!("**@foo", ".a@foo");
    }
}
