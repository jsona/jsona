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

/// Unescape all supported sequences found in [Escape](Escape).
///
/// If it fails, the index of failure is returned.
pub fn unescape(s: &str) -> Result<String, usize> {
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
            Unknown => return Err(lexer.span().end),
            UnEscaped => {
                new_s += lexer.slice();
            }
        }
    }

    Ok(new_s + lexer.remainder())
}

/// Same as unescape, but doesn't create a new
/// unescaped string, and returns all invalid escape indices.
pub fn check_escape(s: &str) -> Result<(), Vec<usize>> {
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
