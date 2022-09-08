use logos::{Lexer, Logos};

/// Escaping based on:
///
/// \b         - backspace       (U+0008)
/// \t         - tab             (U+0009)
/// \n         - linefeed        (U+000A)
/// \f         - form feed       (U+000C)
/// \r         - carriage return (U+000D)
/// \"         - quote           (U+0022)
/// \\         - backslash       (U+005C)
/// \uXXXX     - unicode         (U+XXXX)
/// \UXXXXXXXX - unicode         (U+XXXXXXXX)
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Escape {
    #[token(r#"\0"#)]
    Null,

    #[token(r#"\b"#)]
    Backspace,

    #[token(r#"\t"#)]
    Tab,

    #[regex(r#"(\\\s*\n)|(\\\s*\r\n)"#)]
    Newline,

    #[token(r#"\n"#)]
    LineFeed,

    #[token(r#"\f"#)]
    FormFeed,

    #[token(r#"\r"#)]
    CarriageReturn,

    #[token(r#"\""#)]
    DoubleQuote,

    #[token(r#"\'"#)]
    SingleQuote,

    #[token(r#"\`"#)]
    BacktickQuote,

    #[token(r#"\\"#)]
    Backslash,

    #[regex(r#"\\x[0-9A-Fa-f_][0-9A-Fa-f_]"#)]
    Hex,

    // Same thing repeated 4 times, but the {n} repetition syntax is not supported by Logos
    #[regex(r#"\\u[0-9A-Fa-f_][0-9A-Fa-f_][0-9A-Fa-f_][0-9A-Fa-f_]"#)]
    Unicode,

    // Same thing repeated 8 times, but the {n} repetition syntax is not supported by Logos
    #[regex(r#"\\u\{([0-9A-Fa-f_])+\}"#)]
    UnicodeLarge,

    #[regex(r#"\\."#)]
    Unknown,

    #[error]
    UnEscaped,
}
use Escape::*;

/// Remove quote and unescape all supported sequences found in [Escape](Escape).
///
/// If it fails, the index of failure is returned.
pub fn unquote(s: &str) -> Result<String, usize> {
    let mut new_s = String::with_capacity(s.len());
    let mut lexer: Lexer<Escape> = Lexer::new(s);

    while let Some(t) = lexer.next() {
        match t {
            Null => new_s += "\u{0000}",
            Backspace => new_s += "\u{0008}",
            Tab => new_s += "\u{0009}",
            LineFeed => new_s += "\u{000A}",
            FormFeed => new_s += "\u{000C}",
            CarriageReturn => new_s += "\u{000D}",
            DoubleQuote => new_s += "\u{0022}",
            SingleQuote => new_s += "\u{0027}",
            BacktickQuote => new_s += "\u{0060}",
            Backslash => new_s += "\u{005C}",
            Newline => {}
            Hex | Unicode => {
                new_s += &std::char::from_u32(
                    u32::from_str_radix(&lexer.slice()[2..], 16).map_err(|_| lexer.span().start)?,
                )
                .ok_or(lexer.span().start)?
                .to_string();
            }
            UnicodeLarge => {
                new_s += &std::char::from_u32(
                    u32::from_str_radix(&lexer.slice()[3..(lexer.slice().len() - 1)], 16)
                        .map_err(|_| lexer.span().start)?,
                )
                .ok_or(lexer.span().start)?
                .to_string();
            }
            Unknown => return Err(lexer.span().start),
            UnEscaped => {
                new_s += lexer.slice();
            }
        }
    }
    let output = new_s + lexer.remainder();
    let (unquote_output, quote_symbol) = strip_quote(&output);
    if quote_symbol.is_none() {
        Ok(output)
    } else {
        Ok(unquote_output.to_string())
    }
}

pub fn strip_quote(s: &str) -> (&str, Option<char>) {
    if s.len() < 2 {
        return (s, None);
    }
    if let Some(s) = s.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return (s, Some('"'));
    }
    if let Some(s) = s.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        return (s, Some('\''));
    }
    if let Some(s) = s.strip_prefix('`').and_then(|v| v.strip_suffix('`')) {
        return (s, Some('`'));
    }
    (s, None)
}

/// Same as unescape, but doesn't create a new
/// unescaped string, and returns all invalid escape indices.
pub fn validate_quote(s: &str) -> Result<(), Vec<usize>> {
    let mut lexer: Lexer<Escape> = Lexer::new(s);
    let mut invalid = Vec::new();

    while let Some(t) = lexer.next() {
        match t {
            Null => {}
            Backspace => {}
            Tab => {}
            LineFeed => {}
            FormFeed => {}
            CarriageReturn => {}
            SingleQuote => {}
            DoubleQuote => {}
            BacktickQuote => {}
            Backslash => {}
            Newline => {}
            Hex | Unicode => {
                let char_val = match u32::from_str_radix(&lexer.slice()[2..], 16) {
                    Ok(v) => v,
                    Err(_) => {
                        invalid.push(lexer.span().start);
                        continue;
                    }
                };

                match std::char::from_u32(char_val) {
                    None => {
                        invalid.push(lexer.span().start);
                    }
                    Some(_) => {}
                };
            }
            UnicodeLarge => {
                let char_val =
                    match u32::from_str_radix(&lexer.slice()[3..(lexer.slice().len() - 1)], 16) {
                        Ok(v) => v,
                        Err(_) => {
                            invalid.push(lexer.span().start);
                            continue;
                        }
                    };

                match std::char::from_u32(char_val) {
                    None => {
                        invalid.push(lexer.span().start);
                    }
                    Some(_) => {}
                };
            }
            Unknown => invalid.push(lexer.span().start),
            UnEscaped => {}
        }
    }

    if invalid.is_empty() {
        Ok(())
    } else {
        Err(invalid)
    }
}

pub fn quote(s: &str, force: bool) -> String {
    let quote_char = check_quote(s);
    let mut output = String::new();
    let add_quote = |output: &mut String| match quote_char {
        Some(c) => {
            output.push(c);
        }
        None => {
            if force {
                output.push('"');
            }
        }
    };
    add_quote(&mut output);
    for c in s.chars() {
        match c {
            '\0' => {
                output.push_str("\\0");
            }
            '\u{8}' => {
                output.push_str("\\b");
            }
            '\u{b}' => {
                output.push_str("\\b");
            }
            '\u{c}' => {
                output.push_str("\\f");
            }
            '\\' => {
                output.push_str("\\\\");
            }
            '\t' => {
                output.push_str("\\t");
            }
            '\n' => {
                if quote_char == Some('`') {
                    output.push(c);
                } else {
                    output.push_str("\\n");
                }
            }
            '\r' => {
                if quote_char == Some('`') {
                    output.push(c);
                } else {
                    output.push_str("\\r");
                }
            }
            '\'' => {
                if quote_char == Some('\'') {
                    output.push_str("\\'");
                } else {
                    output.push(c);
                }
            }
            '"' => {
                if quote_char == Some('"') {
                    output.push_str("\\\"");
                } else {
                    output.push(c);
                }
            }
            '`' => {
                if quote_char == Some('`') {
                    output.push_str("\\`");
                } else {
                    output.push(c);
                }
            }
            _ => {
                output.push(c);
            }
        }
    }
    add_quote(&mut output);
    output
}

pub fn check_quote(s: &str) -> Option<char> {
    let mut backtick = false;
    let mut plain = true;
    let mut single = true;
    let mut double = true;
    for c in s.chars() {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            plain = false;
        }
        match c {
            '\n' | '\r' => {
                backtick = true;
            }
            '\"' => {
                double = false;
            }
            '\'' => {
                single = false;
            }
            _ => {}
        }
    }
    if plain {
        return None;
    }
    if backtick {
        return Some('`');
    }
    if double {
        return Some('"');
    }
    if single {
        return Some('\'');
    }
    Some('"')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quote() {
        assert_eq!(strip_quote(""), ("", None));
        assert_eq!(strip_quote("'"), ("'", None));
        assert_eq!(strip_quote("abc"), ("abc", None));
        assert_eq!(strip_quote("'abc'"), ("abc", Some('\'')));
        assert_eq!(strip_quote("`abc`"), ("abc", Some('`')));
        assert_eq!(strip_quote(r#""abc""#), ("abc", Some('"')));
        assert_eq!(strip_quote(r#"'abc""#), (r#"'abc""#, None));
    }

    #[test]
    fn test_unquote() {
        assert_eq!(&unquote("abc").unwrap(), "abc");
        assert_eq!(&unquote("'abc'").unwrap(), "abc");
        assert_eq!(
            &unquote("\\0\\b\\f\\n\\r\\t\\u000b\\'\\\\\\xA9\\u00A9\\u{2F804}").unwrap(),
            "\0\u{8}\u{c}\n\r\t\u{b}'\\©©你"
        );
        assert_eq!(
            &unquote(r#"\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}"#).unwrap(),
            "\0\u{8}\u{c}\n\r\t\u{b}'\\©©你"
        );
        assert_eq!(unquote("\\w"), Err(0));
        assert_eq!(unquote(r#"\w"#), Err(0));
        assert_eq!(unquote("'\\w'"), Err(1));
        assert_eq!(unquote(r#"'\w'"#), Err(1));
    }

    #[test]
    fn test_validate_quote() {
        assert!(validate_quote("abc").is_ok());
        assert!(validate_quote("`abc`").is_ok());
        assert!(validate_quote("\\0\\b\\f\\n\\r\\t\\u000b\\'\\\\\\xA9\\u00A9\\u{2F804}").is_ok());
        assert!(validate_quote(r#"\0\b\f\n\r\t\u000b\'\\\xA9\u00A9\u{2F804}"#).is_ok());
        assert_eq!(validate_quote(r#"'\w\k'"#), Err(vec![1, 3]));
    }

    #[test]
    fn test_check_quote() {
        assert_eq!(check_quote("abc"), None);
        assert_eq!(check_quote("abc\n"), Some('`'));
        assert_eq!(check_quote("abc\r\n"), Some('`'));
        assert_eq!(check_quote("abc\r\ndef"), Some('`'));
        assert_eq!(check_quote("abc'def"), Some('"'));
        assert_eq!(check_quote(r#"abc"def"#), Some('\''));
        assert_eq!(check_quote(r#"abc"def'ijk"#), Some('"'));
    }

    #[test]
    fn test_quote() {
        assert_eq!(&quote("abc", false), "abc");
        assert_eq!(&quote("abc", true), "\"abc\"");
        assert_eq!(&quote("abc-def", false), "\"abc-def\"");
        assert_eq!(&quote("abc'def", false), "\"abc'def\"");
        assert_eq!(&quote("abc\"def", false), "'abc\"def'");
        assert_eq!(&quote("abc\ndef", false), "`abc\ndef`");
        assert_eq!(&quote("abc\r\ndef", false), "`abc\r\ndef`");
    }
}
