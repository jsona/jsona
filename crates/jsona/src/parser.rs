//! JSONA document to syntax tree parsing.

use crate::dom;
use crate::syntax::{SyntaxKind, SyntaxKind::*, SyntaxNode};
use crate::util::check_escape;
use logos::{Lexer, Logos};
use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};
use std::collections::HashSet;

macro_rules! with_node {
    ($builder:expr, $kind:ident, $($content:tt)*) => {
        {
            $builder.start_node($kind.into());
            let res = $($content)*;
            $builder.finish_node();
            res
        }
    };
}

/// A syntax error that can occur during parsing.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Error {
    /// The span of the error.
    pub range: TextRange,

    /// Human-friendly error message.
    pub message: String,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:?})", &self.message, &self.range)
    }
}
impl std::error::Error for Error {}

/// Parse a JSONA document into a [Rowan green tree](rowan::GreenNode).
///
/// The parsing will not stop at unexpected or invalid tokens.
/// Instead errors will be collected with their character offsets and lengths,
/// and the invalid token(s) will have the `ERROR` kind in the final tree.
///
/// The parser will also validate comment and string contents, looking for
/// invalid escape sequences and invalid characters.
/// These will also be reported as syntax errors.
///
/// This does not check for semantic errors such as duplicate keys.
pub fn parse(source: &str) -> Parse {
    Parser::new(source).parse()
}

/// A hand-written parser that uses the Logos lexer
/// to tokenize the source, then constructs
/// a Rowan green tree from them.
pub(crate) struct Parser<'p> {
    current_token: Option<SyntaxKind>,
    lexer: Lexer<'p, SyntaxKind>,
    builder: GreenNodeBuilder<'p>,
    errors: Vec<Error>,
    annotation_scope: bool,
    parse_keys_mode: bool,
}

/// This is just a convenience type during parsing.
/// It allows using "?", making the code cleaner.
type ParserResult<T> = Result<T, ()>;

impl<'p> Parser<'p> {
    pub(crate) fn new(source: &'p str) -> Self {
        Parser {
            current_token: None,
            lexer: SyntaxKind::lexer(source),
            builder: Default::default(),
            errors: Default::default(),
            annotation_scope: false,
            parse_keys_mode: false,
        }
    }

    pub(crate) fn parse_keys_only(mut self) -> Parse {
        self.parse_keys_mode = true;
        let _ = with_node!(self.builder, KEYS, self.parse_keys());

        Parse {
            green_node: self.builder.finish(),
            errors: self.errors,
        }
    }

    fn parse(mut self) -> Parse {
        let _ = with_node!(self.builder, VALUE, self.parse_root());

        Parse {
            green_node: self.builder.finish(),
            errors: self.errors,
        }
    }

    fn parse_root(&mut self) -> ParserResult<()> {
        self.parse_value()?;
        self.parse_annotations()?;
        self.must_peek_eof()
    }

    fn parse_annotations(&mut self) -> ParserResult<()> {
        if let Ok(ANNOATION_KEY) = self.peek_token() {
            self.builder.start_node(ANNOTATIONS.into());
            while let Ok(ANNOATION_KEY) = self.peek_token() {
                if self.lexer.slice().len() == 1 {
                    self.report_error("invalid annotation key");
                }
                let _ = with_node!(self.builder, ANNOTATION_PROPERTY, self.parse_anno_entry());
            }
            self.builder.finish_node();
        }
        Ok(())
    }

    fn parse_anno_entry(&mut self) -> ParserResult<()> {
        self.must_token_or(ANNOATION_KEY, r#"expected annotation key"#)?;
        if self.annotation_scope {
            self.report_error("nested annotation");
        }
        if let Ok(PARENTHESES_START) = self.peek_token() {
            self.annotation_scope = true;
            let ret = with_node!(self.builder, ANNOTATION_VALUE, self.parse_anno_value());
            self.annotation_scope = false;
            ret?;
        }
        Ok(())
    }

    fn parse_anno_value(&mut self) -> ParserResult<()> {
        self.must_token_or(PARENTHESES_START, r#"expected "(""#)?;
        if PARENTHESES_END == self.peek_token()? {
            self.must_token_or(PARENTHESES_END, r#"expected ")""#)?;
            return Ok(());
        }
        let ret = with_node!(self.builder, VALUE, self.parse_value());
        self.must_token_or(PARENTHESES_END, r#"expected ")""#)?;
        ret
    }

    fn parse_property(&mut self) -> ParserResult<bool> {
        with_node!(self.builder, KEY, self.parse_key())?;
        let _ = self.must_token_or(COLON, r#"expected ":""#);
        let ret = with_node!(self.builder, VALUE, self.parse_value_with_annotations());
        Ok(ret.ok().unwrap_or_default())
    }

    fn parse_value(&mut self) -> ParserResult<()> {
        let t = self.must_peek_token()?;
        match t {
            BRACE_START => {
                with_node!(self.builder, OBJECT, self.parse_object())
            }
            BRACKET_START => {
                with_node!(self.builder, ARRAY, self.parse_array())
            }
            NULL | BOOL => with_node!(self.builder, SCALAR, self.consume_current_token()),
            INTEGER => {
                // This could've been done more elegantly probably.
                if (self.lexer.slice().starts_with('0') && self.lexer.slice() != "0")
                    || (self.lexer.slice().starts_with("+0") && self.lexer.slice() != "+0")
                    || (self.lexer.slice().starts_with("-0") && self.lexer.slice() != "-0")
                {
                    self.consume_error_token("zero-padded integers are not allowed")
                } else if !validate_underscore_integer(self.lexer.slice(), 10) {
                    self.consume_error_token("invalid underscores")
                } else {
                    with_node!(self.builder, SCALAR, self.consume_current_token())
                }
            }
            INTEGER_BIN => {
                if !validate_underscore_integer(self.lexer.slice(), 2) {
                    self.consume_error_token("invalid underscores")
                } else {
                    with_node!(self.builder, SCALAR, self.consume_current_token())
                }
            }
            INTEGER_HEX => {
                if !validate_underscore_integer(self.lexer.slice(), 16) {
                    self.consume_error_token("invalid underscores")
                } else {
                    with_node!(self.builder, SCALAR, self.consume_current_token())
                }
            }
            INTEGER_OCT => {
                if !validate_underscore_integer(self.lexer.slice(), 8) {
                    self.consume_error_token("invalid underscores")
                } else {
                    with_node!(self.builder, SCALAR, self.consume_current_token())
                }
            }
            FLOAT => {
                let int_slice = if self.lexer.slice().contains('.') {
                    self.lexer.slice().split('.').next().unwrap()
                } else {
                    self.lexer.slice().split('e').next().unwrap()
                };

                if (int_slice.starts_with('0') && int_slice != "0")
                    || (int_slice.starts_with("+0") && int_slice != "+0")
                    || (int_slice.starts_with("-0") && int_slice != "-0")
                {
                    self.consume_error_token("zero-padded numbers are not allowed")
                } else if !validate_underscore_integer(self.lexer.slice(), 10) {
                    self.consume_error_token("invalid underscores")
                } else {
                    with_node!(self.builder, SCALAR, self.consume_current_token())
                }
            }
            DOUBLE_QUOTE | SINGLE_QUOTE => {
                self.validate_string();
                with_node!(self.builder, SCALAR, self.consume_current_token())
            }
            BACKTICK_QUOTE => {
                self.validate_backtick();
                with_node!(self.builder, SCALAR, self.consume_current_token())
            }
            COMMA => {
                self.report_error("expected value");
                Err(())
            }
            _ => self.consume_error_token("expected value"),
        }
    }

    fn parse_value_with_annotations(&mut self) -> ParserResult<bool> {
        self.parse_value()?;
        let mut has_comma = false;
        if let Ok(COMMA) = self.peek_token() {
            has_comma = true;
            self.consume_current_token()?;
        }
        self.parse_annotations()?;
        Ok(has_comma)
    }

    fn parse_object(&mut self) -> ParserResult<()> {
        self.must_token_or(BRACE_START, r#"expected "{""#)?;
        self.parse_annotations()?;
        let mut needs_comma = false;

        while let Ok(t) = self.must_peek_token() {
            match t {
                BRACE_END => {
                    return self.consume_current_token();
                }
                COMMA => {
                    if needs_comma {
                        needs_comma = false;
                        self.consume_current_token()?;
                    } else {
                        let _ = self.consume_error_token(r#"unexpected ",""#);
                    }
                }
                _ => {
                    if needs_comma {
                        self.point_error(r#"expected ",""#);
                    }
                    let ret = with_node!(self.builder, PROPERTY, self.parse_property());
                    if let Ok(has_comma) = ret {
                        needs_comma = !has_comma;
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_array(&mut self) -> ParserResult<()> {
        self.must_token_or(BRACKET_START, r#"expected "[""#)?;
        let _ = self.parse_annotations();
        let mut needs_comma = false;

        while let Ok(t) = self.must_peek_token() {
            match t {
                BRACKET_END => {
                    return self.consume_current_token();
                }
                COMMA => {
                    if needs_comma {
                        needs_comma = false;
                        self.consume_current_token()?;
                    } else {
                        let _ = self.consume_error_token(r#"unexpected ",""#);
                    }
                }
                _ => {
                    if needs_comma {
                        self.point_error(r#"expected ",""#);
                    }
                    let ret = with_node!(self.builder, VALUE, self.parse_value_with_annotations());
                    needs_comma = !ret.ok().unwrap_or_default();
                }
            }
        }

        Ok(())
    }

    fn parse_keys(&mut self) -> ParserResult<()> {
        let mut first = true;
        let mut after_dot = false;
        let mut exist_annotation_key = false;
        loop {
            let t = match self.peek_token() {
                Ok(token) => token,
                Err(_) => {
                    if !after_dot {
                        return Ok(());
                    }
                    return self.consume_error_token("unexpected EOF");
                }
            };

            match t {
                ANNOATION_KEY => {
                    if after_dot || exist_annotation_key {
                        return self.consume_error_token("unexpected annotation key");
                    } else {
                        self.consume_current_token()?;
                        exist_annotation_key = true;
                        after_dot = false;
                        first = false;
                    }
                }
                PERIOD => {
                    if after_dot {
                        return self.consume_error_token(r#"unexpected ".""#);
                    } else {
                        self.consume_current_token()?;
                        after_dot = true;
                    }
                }
                FLOAT => {
                    let value = self.lexer.slice();
                    if value.starts_with(['+', '-']) {
                        return self.consume_error_token("unexpect identifier");
                    } else {
                        let mut dot = false;
                        for (i, s) in value.split('.').enumerate() {
                            if s.is_empty() {
                                if i == 0 && after_dot {
                                    return self.consume_error_token(r#"unexpect ".""#);
                                }
                                self.consume_token(PERIOD, ".");
                                dot = true;
                            } else {
                                self.consume_token(IDENT, s);
                                dot = false;
                            }
                        }
                        if dot {
                            after_dot = true;
                        }
                        self.next_token();
                    }
                }
                BRACKET_START => {
                    self.consume_current_token()?;

                    self.parse_key()?;

                    let token = self.peek_token()?;

                    if !matches!(token, BRACKET_END) {
                        self.consume_error_token(r#"expected "]""#)?;
                    }
                    self.consume_current_token()?;

                    after_dot = false;
                }
                _ => {
                    if after_dot || first {
                        match self.parse_key() {
                            Ok(_) => {}
                            Err(_) => {
                                self.report_error("expected identifier");
                                return Err(());
                            }
                        }
                        after_dot = false;
                        first = false;
                    } else {
                        return self.consume_error_token(r#"expect ".""#);
                    }
                }
            };
        }
    }

    fn parse_key(&mut self) -> ParserResult<()> {
        let t = self.must_peek_token()?;

        match t {
            IDENT => self.consume_current_token(),
            IDENT_WITH_GLOB if self.parse_keys_mode => {
                if let Err(err_indices) = validates::glob(self.lexer.slice()) {
                    for e in err_indices {
                        let span = self.lexer.span();
                        self.add_error(&Error {
                            range: TextRange::new(
                                TextSize::from((span.start + e) as u32),
                                TextSize::from((span.start + e) as u32),
                            ),
                            message: "invalid glob".into(),
                        });
                    }
                };
                self.consume_current_token()
            }
            NULL | BOOL => self.consume_current_token(),
            INTEGER_HEX | INTEGER_BIN | INTEGER_OCT => self.consume_current_token(),
            INTEGER => {
                if self.lexer.slice().starts_with('+') {
                    Err(())
                } else {
                    self.consume_current_token()
                }
            }
            SINGLE_QUOTE | DOUBLE_QUOTE => {
                self.validate_string();
                self.consume_current_token()
            }
            FLOAT if !self.parse_keys_mode => {
                if self.lexer.slice().starts_with('0') {
                    self.consume_error_token("zero-padded numbers are not allowed")
                } else if self.lexer.slice().starts_with('+') {
                    Err(())
                } else {
                    self.consume_current_token()
                }
            }
            _ => self.consume_error_token("expected identifier"),
        }
    }

    fn must_peek_token(&mut self) -> ParserResult<SyntaxKind> {
        match self.peek_token() {
            Ok(t) => Ok(t),
            Err(_) => {
                self.report_error("unexpected EOF");
                Err(())
            }
        }
    }

    fn must_peek_eof(&mut self) -> ParserResult<()> {
        match self.peek_token() {
            Ok(_) => {
                self.report_error("expected EOF");
                Err(())
            }
            Err(_) => Ok(()),
        }
    }

    fn must_token_or(&mut self, kind: SyntaxKind, message: &str) -> ParserResult<()> {
        let t = self.must_peek_token()?;
        if kind == t {
            self.consume_current_token()
        } else {
            self.report_error(message);
            Err(())
        }
    }

    fn consume_current_token(&mut self) -> ParserResult<()> {
        match self.peek_token() {
            Err(_) => Err(()),
            Ok(token) => {
                self.consume_token(token, self.lexer.slice());
                Ok(())
            }
        }
    }

    fn consume_error_token(&mut self, message: &str) -> ParserResult<()> {
        self.report_error(message);

        self.consume_token(ERROR, self.lexer.slice());

        Err(())
    }

    fn peek_token(&mut self) -> ParserResult<SyntaxKind> {
        if self.current_token.is_none() {
            self.next_token();
        }

        self.current_token.ok_or(())
    }

    fn next_token(&mut self) {
        self.current_token = None;
        while let Some(token) = self.lexer.next() {
            match token {
                LINE_COMMENT | BLOCK_COMMENT => {
                    let multiline = token == BLOCK_COMMENT;
                    if let Err(err_indices) = validates::comment(self.lexer.slice(), multiline) {
                        for e in err_indices {
                            let span = self.lexer.span();
                            self.add_error(&Error {
                                range: TextRange::new(
                                    TextSize::from((span.start + e) as u32),
                                    TextSize::from((span.start + e) as u32),
                                ),
                                message: "invalid character in comment".into(),
                            });
                        }
                    };

                    self.consume_token(token, self.lexer.slice());
                }
                WHITESPACE | NEWLINE => {
                    self.consume_token(token, self.lexer.slice());
                }
                ERROR => {
                    let _ = self.consume_error_token("unexpected token");
                }
                _ => {
                    self.current_token = Some(token);
                    break;
                }
            }
        }
    }

    fn consume_token(&mut self, kind: SyntaxKind, text: &str) {
        self.builder.token(kind.into(), text);
        self.current_token = None;
    }

    fn report_error(&mut self, message: &str) {
        let span = self.lexer.span();

        let err = Error {
            range: TextRange::new(
                TextSize::from(span.start as u32),
                TextSize::from(span.end as u32),
            ),
            message: message.into(),
        };
        self.add_error(&err);
    }

    fn point_error(&mut self, message: &str) {
        let span = self.lexer.span();
        let point = TextSize::from(span.start.saturating_sub(1) as u32);
        let err = Error {
            range: TextRange::new(point, point),
            message: message.into(),
        };
        self.add_error(&err);
    }

    fn add_error(&mut self, e: &Error) {
        if let Some(last_err) = self.errors.last_mut() {
            if last_err.range == e.range {
                return;
            }
        }
        self.errors.push(e.clone());
    }

    fn validate_string(&mut self) {
        let mut indexes: HashSet<usize> = HashSet::default();

        if let Err(err_indices) = validates::string(self.lexer.slice()) {
            indexes.extend(err_indices);
        };
        if let Err(err_indices) = check_escape(self.lexer.slice()) {
            indexes.extend(err_indices);
        };
        let span = self.lexer.span();
        for e in indexes {
            self.add_error(&Error {
                range: TextRange::new(
                    TextSize::from((span.start + e) as u32),
                    TextSize::from((span.start + e + 1) as u32),
                ),
                message: "invalid character in string".into(),
            });
        }
    }
    fn validate_backtick(&mut self) {
        if let Err(err_indices) = validates::backtick_string(self.lexer.slice()) {
            for e in err_indices {
                let span = self.lexer.span();
                self.add_error(&Error {
                    range: TextRange::new(
                        TextSize::from((span.start + e) as u32),
                        TextSize::from((span.start + e + 1) as u32),
                    ),
                    message: "invalid character in string".into(),
                });
            }
        };
    }
}

fn validate_underscore_integer(s: &str, radix: u32) -> bool {
    if s.starts_with('_') || s.ends_with('_') {
        return false;
    }

    let mut prev_char = 0 as char;

    for c in s.chars() {
        if c == '_' && !prev_char.is_digit(radix) {
            return false;
        }
        if !c.is_digit(radix) && prev_char == '_' {
            return false;
        }
        prev_char = c;
    }

    true
}

/// The final results of a parsing.
/// It contains the green tree, and
/// the errors that ocurred during parsing.
#[derive(Debug, Clone)]
pub struct Parse {
    pub green_node: GreenNode,
    pub errors: Vec<Error>,
}

impl Parse {
    /// Turn the parse into a syntax node.
    pub fn into_syntax(self) -> SyntaxNode {
        SyntaxNode::new_root(self.green_node)
    }
    /// Turn the parse into a DOM tree.
    ///
    /// Any semantic errors that occur will be collected
    /// in the returned DOM node.
    pub fn into_dom(self) -> dom::Node {
        dom::from_syntax(self.into_syntax().into())
    }
}

pub(crate) mod validates {
    pub(crate) fn comment(s: &str, multiline: bool) -> Result<(), Vec<usize>> {
        let mut err_indices = Vec::new();

        for (i, c) in s.chars().enumerate() {
            if multiline {
                if c != '\t' && c != '\n' && c != '\r' && c.is_control() {
                    err_indices.push(i);
                }
            } else if c != '\t' && c.is_control() {
                err_indices.push(i);
            }
        }

        if err_indices.is_empty() {
            Ok(())
        } else {
            Err(err_indices)
        }
    }

    pub(crate) fn string(s: &str) -> Result<(), Vec<usize>> {
        let mut err_indices = Vec::new();

        let mut index = 0;
        for c in s.chars() {
            if c != '\t' && c.is_ascii_control() {
                err_indices.push(index);
            }
            index += c.len_utf8();
        }

        if err_indices.is_empty() {
            Ok(())
        } else {
            Err(err_indices)
        }
    }

    pub(crate) fn backtick_string(s: &str) -> Result<(), Vec<usize>> {
        let mut err_indices = Vec::new();

        let mut index = 0;
        for c in s.chars() {
            if c != '\t' && c != '\n' && c != '\r' && c.is_ascii_control() {
                err_indices.push(index);
            }
            index += c.len_utf8();
        }

        if err_indices.is_empty() {
            Ok(())
        } else {
            Err(err_indices)
        }
    }

    pub(crate) fn glob(s: &str) -> Result<(), Vec<usize>> {
        let mut err_indices = Vec::new();

        if s == "*" || s == "**" {
            return Ok(());
        }
        if let Some(i) = s.find("**") {
            err_indices.push(i);
        }
        if err_indices.is_empty() {
            Ok(())
        } else {
            Err(err_indices)
        }
    }
}
