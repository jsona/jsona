//! Declaration of the syntax tokens and lexer implementation.

#![allow(non_camel_case_types)]

use logos::{Lexer, Logos};

/// Enum containing all the tokens in a syntax tree.
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    #[regex(r"([ \t])+")]
    WHITESPACE = 0,

    #[regex(r"(\n|\r\n)+")]
    NEWLINE,

    #[regex(r"/\*", lex_comment_block)]
    BLOCK_COMMENT,

    #[regex(r"//[^\n]*")]
    LINE_COMMENT,

    #[regex(r"[A-Za-z0-9_]+", priority = 2)]
    IDENT,

    #[regex(r"@[A-Za-z0-9_]+")]
    ANNOATION_KEY,

    /// Not part of the regular JSONA syntax, only used to allow
    /// glob patterns in keys.
    #[regex(r"[*?A-Za-z0-9_]+")]
    IDENT_WITH_GLOB,

    #[token(".")]
    PERIOD,

    #[token(",")]
    COMMA,

    #[token(":")]
    COLON,

    #[regex(r#"'"#, lex_single_quote)]
    SINGLE_QUOTE,

    #[regex(r#"""#, lex_double_quote)]
    DOUBLE_QUOTE,

    #[regex(r#"`"#, lex_backtick_quote)]
    BACKTICK_QUOTE,

    #[regex(r"[+-]?[0-9_]+", priority = 4)]
    INTEGER,

    #[regex(r"0x[0-9A-Fa-f_]+")]
    INTEGER_HEX,

    #[regex(r"0o[0-7_]+")]
    INTEGER_OCT,

    #[regex(r"0b(0|1|_)+")]
    INTEGER_BIN,

    #[regex(
        r"[-+]?((([0-9_]+)?(\.[0-9_]+)|([0-9_]+\.)([0-9_]+)?)?([eE][+-]?[0-9_]+)?|nan|inf)",
        priority = 3
    )]
    FLOAT,

    #[regex(r"true|false")]
    BOOL,

    #[token("null")]
    NULL,

    #[token("(")]
    PARENTHESES_START,

    #[token(")")]
    PARENTHESES_END,

    #[token("[")]
    BRACKET_START,

    #[token("]")]
    BRACKET_END,

    #[token("{")]
    BRACE_START,

    #[token("}")]
    BRACE_END,

    // composite types
    KEY,
    SCALAR,
    PROPERTY,
    OBJECT,
    ARRAY,

    ANNOTATION_PROPERTY,
    ANNOTATION_VALUE,

    #[error]
    ERROR,

    KEYS,
    ANNOTATIONS,
    VALUE,
}

impl SyntaxKind {
    pub fn is_comment(self) -> bool {
        use SyntaxKind::*;
        matches!(self, LINE_COMMENT | BLOCK_COMMENT)
    }

    pub fn is_ws(self) -> bool {
        use SyntaxKind::*;
        matches!(self, WHITESPACE | NEWLINE)
    }

    pub fn is_compose(self) -> bool {
        use SyntaxKind::*;
        matches!(self, OBJECT | ARRAY)
    }

    pub fn is_key(self) -> bool {
        use SyntaxKind::*;
        matches!(
            self,
            IDENT
                | IDENT_WITH_GLOB
                | NULL
                | BOOL
                | INTEGER_HEX
                | INTEGER_BIN
                | INTEGER_OCT
                | INTEGER
                | SINGLE_QUOTE
                | DOUBLE_QUOTE
                | FLOAT
        )
    }

    pub fn is_ws_or_comment(self) -> bool {
        self.is_ws() || self.is_comment()
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {}
impl rowan::Language for Lang {
    type Kind = SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= SyntaxKind::VALUE as u16);
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<Lang>;
pub type SyntaxToken = rowan::SyntaxToken<Lang>;
pub type SyntaxElement = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;

pub fn stringify_syntax(
    indent: usize,
    element: SyntaxElement,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = vec![];
    write_syntax(&mut buf, indent, element)?;
    Ok(std::str::from_utf8(&buf)?.to_string())
}

pub fn write_syntax<T: std::io::Write>(
    w: &mut T,
    indent: usize,
    element: SyntaxElement,
) -> Result<(), Box<dyn std::error::Error>> {
    let kind: SyntaxKind = element.kind();
    write!(w, "{:indent$}", "", indent = indent)?;
    match element {
        rowan::NodeOrToken::Node(node) => {
            writeln!(w, "{:?}@{:?}", kind, node.text_range())?;
            for child in node.children_with_tokens() {
                write_syntax(w, indent + 2, child)?;
            }
        }

        rowan::NodeOrToken::Token(token) => {
            writeln!(w, "{:?}@{:?} {:?}", kind, token.text_range(), token.text())?;
        }
    }
    Ok(())
}

fn lex_comment_block(lex: &mut Lexer<SyntaxKind>) -> bool {
    let remainder: &str = lex.remainder();

    let mut asterisk_found = false;

    let mut total_len = 0;

    for c in remainder.chars() {
        total_len += c.len_utf8();

        if c == '*' {
            asterisk_found = true;
            continue;
        }

        if c == '/' && asterisk_found {
            lex.bump(remainder[0..total_len].as_bytes().len());
            return true;
        }

        asterisk_found = false;
    }
    false
}

fn lex_backtick_quote(lex: &mut Lexer<SyntaxKind>) -> bool {
    lex_string(lex, '`')
}

fn lex_single_quote(lex: &mut Lexer<SyntaxKind>) -> bool {
    lex_string(lex, '\'')
}

fn lex_double_quote(lex: &mut Lexer<SyntaxKind>) -> bool {
    lex_string(lex, '"')
}

fn lex_string(lex: &mut Lexer<SyntaxKind>, quote: char) -> bool {
    let remainder: &str = lex.remainder();
    let mut escaped = false;

    let mut total_len = 0;

    for c in remainder.chars() {
        total_len += c.len_utf8();

        if c == '\\' {
            escaped = !escaped;
            continue;
        }

        if c == quote && !escaped {
            lex.bump(remainder[0..total_len].as_bytes().len());
            return true;
        }

        escaped = false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_lex {
        ($text:literal, $kind:expr) => {
            let mut lex = SyntaxKind::lexer($text);
            assert_eq!(lex.next(), Some($kind));
        };
    }

    #[test]
    fn test_lex() {
        assert_lex!("/* comment */", SyntaxKind::BLOCK_COMMENT);
        assert_lex!("// comment", SyntaxKind::LINE_COMMENT);
        assert_lex!("foo", SyntaxKind::IDENT);
        assert_lex!(r#""I'm a string\u00E9""#, SyntaxKind::DOUBLE_QUOTE);
        assert_lex!(r#"'Say "hello"'"#, SyntaxKind::SINGLE_QUOTE);
        assert_lex!(r#"`hello world`"#, SyntaxKind::BACKTICK_QUOTE);
        assert_lex!("123", SyntaxKind::INTEGER);
        assert_lex!("0xDEADBEEF", SyntaxKind::INTEGER_HEX);
        assert_lex!("0xDE_ADBE", SyntaxKind::INTEGER_HEX);
        assert_lex!("0o4567", SyntaxKind::INTEGER_OCT);
        assert_lex!("0o45_67", SyntaxKind::INTEGER_OCT);
        assert_lex!("0b11010110", SyntaxKind::INTEGER_BIN);
        assert_lex!("0b1101_0110", SyntaxKind::INTEGER_BIN);
        assert_lex!("3.14", SyntaxKind::FLOAT);
        assert_lex!("-.14", SyntaxKind::FLOAT);
        assert_lex!("-3.", SyntaxKind::FLOAT);
        assert_lex!("true", SyntaxKind::BOOL);
        assert_lex!("false", SyntaxKind::BOOL);
        assert_lex!("null", SyntaxKind::NULL);
        assert_lex!("api*", SyntaxKind::IDENT_WITH_GLOB);
        assert_lex!("a?i*", SyntaxKind::IDENT_WITH_GLOB);
        assert_lex!("*", SyntaxKind::IDENT_WITH_GLOB);
        assert_lex!("**", SyntaxKind::IDENT_WITH_GLOB);
    }
}
