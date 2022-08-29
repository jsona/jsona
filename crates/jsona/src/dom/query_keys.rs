use super::from_syntax::query_keys_from_syntax;
use super::node::Key;
use super::{KeyOrIndex, Keys};
use crate::parser::Parser;
use crate::util::glob;

use std::iter::empty;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryKey {
    Index(usize),
    Key(Key),
    GlobIndex(String),
    GlobKey(String),
    AnyRecursive,
}

impl core::fmt::Display for QueryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryKey::Index(v) => write!(f, "[{}]", v),
            QueryKey::Key(v) => {
                if v.is_property() {
                    write!(f, ".{}", v)
                } else {
                    write!(f, "{}", v)
                }
            }
            QueryKey::GlobIndex(v) => write!(f, "[{}]", v),
            QueryKey::GlobKey(v) => write!(f, ".{}", v),
            QueryKey::AnyRecursive => write!(f, "**"),
        }
    }
}

impl QueryKey {
    pub fn is_match(&self, other: &KeyOrIndex) -> bool {
        match self {
            QueryKey::Index(v1) => match other {
                KeyOrIndex::Index(v2) => v1 == v2,
                _ => false,
            },
            QueryKey::Key(v1) => match other {
                KeyOrIndex::Key(v2) => v1 == v2,
                _ => false,
            },
            QueryKey::GlobIndex(v1) => match other {
                KeyOrIndex::Index(v2) => glob(v1, &v2.to_string()),
                _ => false,
            },
            QueryKey::GlobKey(v1) => match other {
                // NOTE: glob key only works on property key
                KeyOrIndex::Key(v2) if v2.is_property() => glob(v1, v2.value()),
                _ => false,
            },
            QueryKey::AnyRecursive => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryKeys {
    dotted: Arc<str>,
    keys: Arc<[QueryKey]>,
    exist_any_recursive: bool,
}

impl QueryKeys {
    pub fn new(keys: impl Iterator<Item = QueryKey>) -> Self {
        let keys: Arc<[QueryKey]> = keys.collect();
        let mut dotted = String::new();
        let mut exist_any_recursive = false;
        for k in keys.iter() {
            match k {
                QueryKey::Index(_)
                | QueryKey::Key(_)
                | QueryKey::GlobIndex(_)
                | QueryKey::GlobKey(_) => {}
                QueryKey::AnyRecursive => {
                    exist_any_recursive = true;
                }
            }
            dotted.push_str(&k.to_string());
        }
        let dotted: Arc<str> = Arc::from(dotted);
        Self {
            keys,
            dotted,
            exist_any_recursive,
        }
    }

    pub fn dotted(&self) -> &str {
        &*self.dotted
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &QueryKey> + DoubleEndedIterator {
        self.keys.iter()
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
            let keys: Vec<&QueryKey> = self.iter().collect();
            let target_keys: Vec<&KeyOrIndex> = other.iter().collect();
            let mut i = 0;
            let mut j = 0;
            'outer: while i < self.len() {
                let key = keys[i];
                match key {
                    QueryKey::Index(_)
                    | QueryKey::Key(_)
                    | QueryKey::GlobIndex(_)
                    | QueryKey::GlobKey(_) => match target_keys.get(j) {
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
                    QueryKey::AnyRecursive => {
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

impl core::fmt::Display for QueryKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.dotted().fmt(f)
    }
}

impl FromStr for QueryKeys {
    type Err = Vec<crate::parser::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s == "." {
            return Ok(QueryKeys::default());
        }
        let p = Parser::new(s).parse_keys_only(true);
        if !p.errors.is_empty() {
            return Err(p.errors);
        }
        Ok(QueryKeys::new(query_keys_from_syntax(
            &p.into_syntax().into(),
        )))
    }
}

impl Default for QueryKeys {
    fn default() -> Self {
        Self::new(empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_parse_query_keys {
        ($v1:literal) => {
            assert_eq!($v1.parse::<QueryKeys>().unwrap().to_string(), $v1);
        };
        ($v1:literal, $v2:literal) => {
            assert_eq!($v1.parse::<QueryKeys>().unwrap().to_string(), $v2);
        };
    }

    macro_rules! assert_match_keys {
        ($v1:literal, $v2:literal) => {
            assert_eq!(
                $v1.parse::<QueryKeys>()
                    .unwrap()
                    .is_match(&$v2.parse::<Keys>().unwrap(), false),
                true
            );
        };
        ($v1:literal, $v2:literal, $v3:literal) => {
            assert_eq!(
                $v1.parse::<QueryKeys>()
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
        assert_parse_query_keys!("");
        assert_parse_query_keys!("[1]");
        assert_parse_query_keys!("[*]");
        assert_parse_query_keys!(".foo");
        assert_parse_query_keys!("foo", ".foo");
        assert_parse_query_keys!(".*");
        assert_parse_query_keys!("**");
        assert_parse_query_keys!("**.**", "**");
        assert_parse_query_keys!(".foo.bar");
        assert_parse_query_keys!(".foo@bar");
        assert_parse_query_keys!("@foo");
        assert_parse_query_keys!("[0].foo");
        assert_parse_query_keys!("[0][1]");
        assert_parse_query_keys!("[*].foo");
        assert_parse_query_keys!("[*][1]");
        assert_parse_query_keys!("[*]@foo");
        assert_parse_query_keys!(".*@foo");
        assert_parse_query_keys!(".foo*");
        assert_parse_query_keys!(".foo.*");
        assert_parse_query_keys!(".foo.*.bar");
        assert_parse_query_keys!(r#".foo."ba-z""#, r#".foo."ba-z""#);
        assert_parse_query_keys!(r#".foo."ba z""#, r#".foo."ba z""#);
        assert_parse_query_keys!(".foo.1");
        assert_parse_query_keys!(".foo.1.baz");
        assert_parse_query_keys!(r#".foo."1".baz"#, ".foo.1.baz");
        assert_parse_query_keys!("*foo", ".*foo");
        assert_parse_query_keys!("**@foo");
        assert_parse_query_keys!("**.*");
    }

    #[test]
    fn test_parse_keys_fails() {
        assert!("..foo".parse::<QueryKeys>().is_err());
        assert!("foo.b-z".parse::<QueryKeys>().is_err());
        assert!("foo.".parse::<QueryKeys>().is_err());
        assert!("foo.b-*".parse::<QueryKeys>().is_err());
        assert!("foo.b**".parse::<QueryKeys>().is_err());
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
