//! Declaration of the syntax tokens and lexer implementation.

#![allow(non_camel_case_types)]

use logos::Logos;

/// Enum containing all the tokens in a syntax tree.
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    #[regex(r"([ \t])+")]
    WHITESPACE = 0,

    #[regex(r"(\n|\r\n)+")]
    NEWLINE,

    #[regex(r"/\*(?:[^*]|\*[^/])*\*/")]
    COMMENT_BLOCK,

    #[regex(r"//[^\n]*")]
    COMMENT_LINE,

    #[regex(r"[A-Za-z0-9_-]+", priority = 2)]
    IDENT,

    #[token(".")]
    PERIOD,

    #[token(",")]
    COMMA,

    #[token(":")]
    COLON,

    #[token("@")]
    AT,

    #[regex(r#"'(?:[^']|\\')*'"#)]
    SINGLE_QUOTE,

    #[regex(r#""(?:[^"]|\\")*""#)]
    DOUBLE_QUOTE,

    #[regex(r#"`(?:[^`]|\\`)*`"#)]
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
    ENTRY,
    OBJECT,
    ARRAY,
    ANNO_VALUE,

    #[error]
    ERROR,

    ANNOS,
    VALUE,
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
        assert_lex!("/* comment */", SyntaxKind::COMMENT_BLOCK);
        assert_lex!("// comment", SyntaxKind::COMMENT_LINE);
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
    }
}
