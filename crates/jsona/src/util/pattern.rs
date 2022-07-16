//! Glob pattern for match keys
use std::fmt;
use std::{error::Error, str::FromStr};

use MatchResult::{EntirePatternDoesntMatch, Match, SubPatternDoesntMatch};
use PatternToken::{AnyChar, AnyRecursiveSequence, AnySequence, Char};

// A pattern parsing error.
#[derive(Debug, Clone)]
#[allow(missing_copy_implementations)]
pub struct PatternError {
    /// The approximate character index of where the error occurred.
    pub pos: usize,

    /// A message describing the error.
    pub msg: &'static str,
}

impl Error for PatternError {
    fn description(&self) -> &str {
        self.msg
    }
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Pattern syntax error near position {}: {}",
            self.pos, self.msg
        )
    }
}

/// A compiled Unix shell style pattern.
///
/// - `?` matches any single character.
///
/// - `*` matches any (possibly empty) sequence of characters.
///
/// - `**` matches the current directory and arbitrary subdirectories. This
///   sequence **must** form a single path component, so both `**a` and `b**`
///   are invalid and will result in an error.  A sequence of more than two
///   consecutive `*` characters is also invalid.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct Pattern {
    original: String,
    tokens: Vec<PatternToken>,
    is_recursive: bool,
    is_glob: bool,
}

/// Show the original glob pattern.
impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.original.fmt(f)
    }
}

impl FromStr for Pattern {
    type Err = PatternError;

    fn from_str(s: &str) -> Result<Self, PatternError> {
        Self::new(s)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum PatternToken {
    Char(char),
    AnyChar,
    AnySequence,
    AnyRecursiveSequence,
}

#[derive(Copy, Clone, PartialEq)]
enum MatchResult {
    Match,
    SubPatternDoesntMatch,
    EntirePatternDoesntMatch,
}

const ERROR_WILDCARDS: &str = "wildcards are either regular `*` or recursive `**`";
const ERROR_RECURSIVE_WILDCARDS: &str = "recursive wildcards must form a single key . component";

impl Pattern {
    /// This function compiles Unix shell style patterns.
    ///
    /// An invalid glob pattern will yield a `PatternError`.
    pub fn new(pattern: &str) -> Result<Self, PatternError> {
        let chars = pattern.chars().collect::<Vec<_>>();
        let mut tokens = Vec::new();
        let mut is_recursive = false;
        let mut i = 0;
        let mut is_glob = false;

        while i < chars.len() {
            match chars[i] {
                '?' => {
                    is_glob = true;
                    tokens.push(AnyChar);
                    i += 1;
                }
                '*' => {
                    is_glob = true;
                    let old = i;

                    while i < chars.len() && chars[i] == '*' {
                        i += 1;
                    }

                    let count = i - old;

                    match count {
                        1 => {
                            tokens.push(AnySequence);
                        }
                        2 => {
                            // ** can only be an entire path component
                            // i.e. a/**/b is valid, but a**/b or a/**b is not
                            // invalid matches are treated literally
                            let is_valid = if i == 2 || is_seperator(chars[i - count - 1]) {
                                // it ends in a '/'
                                if i < chars.len() && is_seperator(chars[i]) {
                                    i += 1;
                                    true
                                // or the pattern ends here
                                // this enables the existing globbing mechanism
                                } else if i == chars.len() {
                                    true
                                // `**` ends in non-separator
                                } else {
                                    return Err(PatternError {
                                        pos: i,
                                        msg: ERROR_RECURSIVE_WILDCARDS,
                                    });
                                }
                            // `**` begins with non-separator
                            } else {
                                return Err(PatternError {
                                    pos: old - 1,
                                    msg: ERROR_RECURSIVE_WILDCARDS,
                                });
                            };

                            if is_valid {
                                // collapse consecutive AnyRecursiveSequence to a
                                // single one

                                let tokens_len = tokens.len();

                                if !(tokens_len > 1
                                    && tokens[tokens_len - 1] == AnyRecursiveSequence)
                                {
                                    is_recursive = true;
                                    tokens.push(AnyRecursiveSequence);
                                }
                            }
                        }
                        _ if count > 2 => {
                            return Err(PatternError {
                                pos: old + 2,
                                msg: ERROR_WILDCARDS,
                            });
                        }
                        _ => {}
                    }
                }
                c => {
                    tokens.push(Char(c));
                    i += 1;
                }
            }
        }

        Ok(Self {
            tokens,
            original: pattern.to_string(),
            is_recursive,
            is_glob,
        })
    }

    /// Return if the given `str` matches this `Pattern` using the default
    /// match options (i.e. `MatchOptions::new()`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use jsona::util::pattern::Pattern;
    ///
    /// assert!(Pattern::new("c?t").unwrap().matches("cat"));
    /// assert!(Pattern::new("d*g").unwrap().matches("doog"));
    /// ```
    pub fn matches(&self, str: &str) -> bool {
        if self.is_glob {
            self.matches_from(true, str.chars(), 0) == Match
        } else {
            self.original == str
        }
    }

    /// Access the original glob pattern.
    pub fn as_str(&self) -> &str {
        &self.original
    }

    fn matches_from(
        &self,
        mut follows_separator: bool,
        mut keys: std::str::Chars,
        i: usize,
    ) -> MatchResult {
        for (ti, token) in self.tokens[i..].iter().enumerate() {
            match *token {
                AnySequence | AnyRecursiveSequence => {
                    // ** must be at the start.
                    debug_assert!(match *token {
                        AnyRecursiveSequence => follows_separator,
                        _ => true,
                    });

                    // Empty match
                    match self.matches_from(follows_separator, keys.clone(), i + ti + 1) {
                        SubPatternDoesntMatch => (), // keep trying
                        m => return m,
                    };

                    while let Some(c) = keys.next() {
                        follows_separator = is_seperator(c);
                        match *token {
                            AnyRecursiveSequence if !follows_separator => continue,
                            AnySequence if follows_separator => return SubPatternDoesntMatch,
                            _ => (),
                        }
                        match self.matches_from(follows_separator, keys.clone(), i + ti + 1) {
                            SubPatternDoesntMatch => (), // keep trying
                            m => return m,
                        }
                    }
                }
                _ => {
                    let c = match keys.next() {
                        Some(c) => c,
                        None => return EntirePatternDoesntMatch,
                    };

                    let is_sep = is_seperator(c);

                    if !match *token {
                        AnyChar if is_sep => false,
                        AnyChar => true,
                        Char(c2) => chars_eq(c, c2, true),
                        AnySequence | AnyRecursiveSequence => unreachable!(),
                    } {
                        return SubPatternDoesntMatch;
                    }
                    follows_separator = is_sep;
                }
            }
        }

        // Iter is fused.
        if keys.next().is_none() {
            Match
        } else {
            SubPatternDoesntMatch
        }
    }
}

fn is_seperator(s: char) -> bool {
    s == '.'
}

/// A helper function to determine if two chars are (possibly case-insensitively) equal.
fn chars_eq(a: char, b: char, case_sensitive: bool) -> bool {
    if !case_sensitive && a.is_ascii() && b.is_ascii() {
        // FIXME: work with non-ascii chars properly (issue #9084)
        a.to_ascii_lowercase() == b.to_ascii_lowercase()
    } else {
        a == b
    }
}

#[cfg(test)]
mod test {
    use super::Pattern;

    #[test]
    fn test_match() {
        assert!(Pattern::new("").unwrap().matches(""));
        assert!(Pattern::new("a*b").unwrap().matches("a_b"));
        assert!(Pattern::new("a*b").unwrap().matches("a_b"));
        assert!(Pattern::new("a*b*c").unwrap().matches("abc"));
        assert!(!Pattern::new("a*b*c").unwrap().matches("abcd"));
        assert!(Pattern::new("a*b*c").unwrap().matches("a_b_c"));
        assert!(Pattern::new("a*b*c").unwrap().matches("a___b___c"));
        assert!(Pattern::new("abc*abc*abc")
            .unwrap()
            .matches("abcabcabcabcabcabcabc"));
        assert!(!Pattern::new("abc*abc*abc")
            .unwrap()
            .matches("abcabcabcabcabcabcabca"));
        assert!(Pattern::new("a*a*a*a*a*a*a*a*a")
            .unwrap()
            .matches("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn test_wildcard_errors() {
        assert!(Pattern::new("a.**b").unwrap_err().pos == 4);
        assert!(Pattern::new("a.bc**").unwrap_err().pos == 3);
        assert!(Pattern::new("a.*****").unwrap_err().pos == 4);
        assert!(Pattern::new("a.b**c**d").unwrap_err().pos == 2);
        assert!(Pattern::new("a**b").unwrap_err().pos == 0);
    }

    #[test]
    fn test_recursive_wildcards() {
        let pat = Pattern::new("some.**.needle.txt").unwrap();
        assert!(pat.matches("some.needle.txt"));
        assert!(pat.matches("some.one.needle.txt"));
        assert!(pat.matches("some.one.two.needle.txt"));
        assert!(pat.matches("some.other.needle.txt"));
        assert!(!pat.matches("some.other.notthis.txt"));

        // a single ** should be valid, for globs
        // Should accept anything
        let pat = Pattern::new("**").unwrap();
        assert!(pat.is_recursive);
        assert!(pat.matches("abcde"));
        assert!(pat.matches(""));
        assert!(pat.matches(".asdf"));
        assert!(pat.matches(".x..asdf"));

        // collapse consecutive wildcards
        let pat = Pattern::new("some.**.**.needle").unwrap();
        assert!(pat.matches("some.needle"));
        assert!(pat.matches("some.one.needle"));
        assert!(pat.matches("some.one.two.needle"));
        assert!(pat.matches("some.other.needle"));
        assert!(!pat.matches("some.other.notthis"));

        // ** can begin the pattern
        let pat = Pattern::new("**.test").unwrap();
        assert!(pat.matches("one.two.test"));
        assert!(pat.matches("one.test"));
        assert!(pat.matches("test"));

        // /** can begin the pattern
        let pat = Pattern::new(".**.test").unwrap();
        assert!(pat.matches(".one.two.test"));
        assert!(pat.matches(".one.test"));
        assert!(pat.matches(".test"));
        assert!(!pat.matches(".one.notthis"));
        assert!(!pat.matches(".notthis"));
    }
}
