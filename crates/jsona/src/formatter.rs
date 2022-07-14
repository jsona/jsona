//! This module is used to format TOML.
//!
//! The formatting can be done on documents that might
//! contain invalid syntax. In that case the invalid part is skipped.

use std::{cell::RefCell, rc::Rc};

use crate::{
    parser,
    syntax::{SyntaxKind::*, SyntaxNode, SyntaxToken},
};

use rowan::{NodeOrToken, TextRange, WalkEvent};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

macro_rules! create_options {
    (
        $(#[$attr:meta])*
        pub struct Options {
            $(
                $(#[$field_attr:meta])*
                pub $name:ident: $ty:ty,
            )+
        }
    ) => {
        $(#[$attr])*
        pub struct Options {
            $(
                $(#[$field_attr])*
                pub $name: $ty,
            )+
        }

        impl Options {
            pub fn update(&mut self, incomplete: OptionsIncomplete) {
                $(
                    if let Some(v) = incomplete.$name {
                        self.$name = v;
                    }
                )+
            }

            pub fn update_camel(&mut self, incomplete: OptionsIncompleteCamel) {
                $(
                    if let Some(v) = incomplete.$name {
                        self.$name = v;
                    }
                )+
            }

            pub fn update_from_str<S: AsRef<str>, I: Iterator<Item = (S, S)>>(
                &mut self,
                values: I,
            ) -> Result<(), OptionParseError> {
                for (key, val) in values {

                    $(
                        if key.as_ref() == stringify!($name) {
                            self.$name =
                                val.as_ref()
                                    .parse()
                                    .map_err(|error| OptionParseError::InvalidValue {
                                        key: key.as_ref().into(),
                                        error: Box::new(error),
                                    })?;

                            continue;
                        }
                    )+

                    return Err(OptionParseError::InvalidOption(key.as_ref().into()));
                }

                Ok(())
            }
        }

        $(#[$attr])*
        #[derive(Default)]
        pub struct OptionsIncomplete {
            $(
                $(#[$field_attr])*
                pub $name: Option<$ty>,
            )+
        }

        impl OptionsIncomplete {
            pub fn from_options(opts: Options) -> Self {
                let mut o = Self::default();

                $(
                    o.$name = Some(opts.$name);
                )+

                o
            }
        }

        $(#[$attr])*
        #[derive(Default)]
        #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
        pub struct OptionsIncompleteCamel {
            $(
                $(#[$field_attr])*
                pub $name: Option<$ty>,
            )+
        }

        impl OptionsIncompleteCamel {
            pub fn from_options(opts: Options) -> Self {
                let mut o = Self::default();

                $(
                    o.$name = Some(opts.$name);
                )+

                o
            }
        }
    };
}

create_options!(
    /// All the formatting options.
    #[derive(Debug, Clone, Eq, PartialEq)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct Options {
        /// Target maximum column width after which
        /// annotations are expanded into new lines.
        ///
        /// This is best-effort and might not be accurate.
        pub column_width: usize,

        /// Indentation to use, should be tabs or spaces
        /// but technically could be anything.
        pub indent_string: String,

        /// Put trailing commas for multiline arrays/objects
        pub trailing_comma: bool,

        /// Add trailing newline to the source.
        pub trailing_newline: bool,
    }
);

#[derive(Debug)]
pub enum OptionParseError {
    InvalidOption(String),
    InvalidValue {
        key: String,
        error: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl core::fmt::Display for OptionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid formatting option: {}",
            match self {
                OptionParseError::InvalidOption(k) => {
                    format!(r#"invalid option "{}""#, k)
                }
                OptionParseError::InvalidValue { key, error } => {
                    format!(r#"invalid value for option "{}": {}"#, key, error)
                }
            }
        )
    }
}

impl std::error::Error for OptionParseError {}

impl Default for Options {
    fn default() -> Self {
        Options {
            column_width: 80,
            indent_string: "  ".into(),
            trailing_comma: true,
            trailing_newline: true,
        }
    }
}

#[derive(Debug, Clone)]
struct Scope {
    options: Rc<Options>,
    level: usize,
    error_ranges: Rc<[TextRange]>,
    formatted: Rc<RefCell<String>>,
    kind: ScopeKind,
}

impl Scope {
    fn enter(&self, kind: ScopeKind) -> Self {
        Self {
            options: self.options.clone(),
            level: self.level + 1,
            error_ranges: self.error_ranges.clone(),
            formatted: self.formatted.clone(),
            kind,
        }
    }
    fn exit(&self) -> Self {
        Self {
            options: self.options.clone(),
            level: self.level - 1,
            error_ranges: self.error_ranges.clone(),
            formatted: self.formatted.clone(),
            kind: self.kind,
        }
    }
    fn write<T: AsRef<str>>(&self, text: T) -> usize {
        let text = text.as_ref();
        let len = text.len();
        self.formatted.borrow_mut().push_str(text);
        len
    }
    fn write_with_ident<T: AsRef<str>>(&self, text: T) -> usize {
        let ident = self.ident();
        let idented_text = format!("{}{}", ident, text.as_ref());
        self.formatted.borrow_mut().push_str(&idented_text);
        idented_text.len()
    }
    fn read(&self) -> String {
        self.formatted.borrow_mut().to_string()
    }
    fn ident(&self) -> String {
        self.options.indent_string.repeat(self.level)
    }
    fn is_last_char(&self, c: char) -> bool {
        self.formatted
            .borrow()
            .chars()
            .last()
            .map(|v| v == c)
            .unwrap_or_default()
    }
    fn remove_last_char(&self) {
        self.formatted.borrow_mut().pop();
    }
    fn is_array_scope(&self) -> bool {
        self.kind == ScopeKind::Array
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Root,
    Array,
    Object,
}

#[derive(Debug, Clone)]
struct Context {
    col_offset: usize,
    last_nodes: Vec<bool>,
}

impl Context {
    fn maybe_insert_space(&mut self, scope: &Scope) {
        if self.col_offset > 0 && !scope.is_last_char(' ') {
            self.col_offset += scope.write(" ");
        }
    }

    fn maybe_insert_comma(&mut self, scope: &Scope) {
        match self.last_nodes.last() {
            Some(&true) => {
                if scope.options.trailing_comma {
                    self.col_offset += scope.write(",");
                }
            }
            Some(&false) => {
                self.col_offset += scope.write(",");
            }
            None => {}
        };
    }

    fn newline(&mut self, scope: &Scope) {
        if self.col_offset > 0 {
            scope.write("\n");
            self.col_offset = 0;
        }
    }
}

/// Parses then formats a JSONA document, skipping ranges that contain syntax errors.
pub fn format(src: &str, options: Options) -> String {
    let p = parser::parse(src);
    let error_ranges = p.errors.iter().map(|err| err.range).collect();
    let trailing_newline = options.trailing_newline;
    let scope = Scope {
        options: Rc::new(options),
        level: 0,
        formatted: Default::default(),
        error_ranges,
        kind: ScopeKind::Root,
    };
    let mut ctx = Context {
        col_offset: 0,
        last_nodes: vec![],
    };
    format_value(scope.clone(), p.into_syntax(), &mut ctx);
    let mut formatted = scope.read();
    if formatted.ends_with('\n') {
        formatted.truncate(formatted.len() - 1);
    }
    if trailing_newline {
        formatted += "\n";
    }
    formatted
}

fn format_value(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != VALUE {
        scope.write(&syntax.to_string());
        return;
    }
    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                SCALAR => format_scalar(scope.clone(), n, ctx),
                OBJECT => format_object(scope.clone(), n, ctx),
                ARRAY => format_array(scope.clone(), n, ctx),
                ANNOTATIONS => format_annotations(scope.clone(), n, ctx),
                _ => {}
            },
            NodeOrToken::Token(t) => match t.kind() {
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }
}

fn format_scalar(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    let text = syntax.to_string();
    if ctx.col_offset == 0 && scope.is_array_scope() {
        ctx.col_offset += scope.write_with_ident(&text);
    } else {
        scope.write(&text);
        ctx.col_offset += text.len();
    }
    if is_multiline(&text) {
        if let Some(offset) = text.split('\n').last().map(|v| v.len()) {
            ctx.col_offset = offset
        }
    }
    ctx.maybe_insert_comma(&scope);
}

fn format_object(mut scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != OBJECT {
        scope.write(&syntax.to_string());
        return;
    }

    let mut is_empty = true;

    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => {
                is_empty = false;
                match n.kind() {
                    ENTRY => {
                        if let Some(c) = ctx.last_nodes.last_mut() {
                            *c = n.next_sibling().is_none();
                        }
                        format_entry(scope.clone(), n, ctx);
                    }
                    ANNOTATIONS => format_annotations(scope.clone(), n, ctx),
                    _ => {}
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                BRACE_START => {
                    if ctx.col_offset == 0 {
                        ctx.col_offset += scope.write_with_ident("{");
                    } else {
                        ctx.col_offset += scope.write("{");
                    }
                    scope = scope.enter(ScopeKind::Object);
                    ctx.last_nodes.push(false);
                }
                BRACE_END => {
                    scope = scope.exit();
                    ctx.last_nodes.pop();
                    if is_empty {
                        if ctx.col_offset == 0 {
                            ctx.col_offset += scope.write_with_ident("}");
                        } else {
                            ctx.col_offset += scope.write("}");
                        }
                    } else {
                        ctx.newline(&scope);
                        ctx.col_offset += scope.write_with_ident("}");
                    }
                }
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }
    ctx.maybe_insert_comma(&scope);
}

fn format_entry(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != ENTRY {
        scope.write(&syntax.to_string());
        return;
    }
    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                KEY => {
                    let text = n.to_string();
                    ctx.newline(&scope);
                    ctx.col_offset += scope.write_with_ident(&text);
                }
                VALUE => format_value(scope.clone(), n, ctx),
                _ => {}
            },
            NodeOrToken::Token(t) => match t.kind() {
                COLON => {
                    ctx.col_offset += scope.write(": ");
                }
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }
}

fn format_array(mut scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != ARRAY {
        scope.write(&syntax.to_string());
        return;
    }

    let mut is_empty = true;

    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => {
                is_empty = false;
                match n.kind() {
                    VALUE => {
                        if let Some(c) = ctx.last_nodes.last_mut() {
                            *c = n.next_sibling().is_none();
                        }
                        format_value(scope.clone(), n, ctx);
                    }
                    ANNOTATIONS => format_annotations(scope.clone(), n, ctx),
                    _ => {}
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                BRACKET_START => {
                    if ctx.col_offset == 0 {
                        ctx.col_offset += scope.write_with_ident("[");
                    } else {
                        ctx.col_offset += scope.write("[");
                    }
                    scope = scope.enter(ScopeKind::Array);
                    ctx.last_nodes.push(false);
                }
                BRACKET_END => {
                    scope = scope.exit();
                    ctx.last_nodes.pop();
                    if is_empty {
                        if ctx.col_offset == 0 {
                            ctx.col_offset += scope.write_with_ident("]");
                        } else {
                            ctx.col_offset += scope.write("]");
                        }
                    } else {
                        if ctx.col_offset > 0 {
                            scope.write("\n");
                            ctx.col_offset = 0;
                        }
                        ctx.col_offset += scope.write_with_ident("]");
                    }
                }
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }

    ctx.maybe_insert_comma(&scope);
}

fn format_annotations(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != ANNOTATIONS {
        scope.write(&syntax.to_string());
        return;
    }

    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => {
                if n.kind() == ANNOTATION_ENTRY {
                    format_annotation_entry(scope.clone(), n, ctx);
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }
}

fn format_annotation_entry(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != ANNOTATION_ENTRY {
        scope.write(&syntax.to_string());
        return;
    }
    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                KEY => {
                    if ctx.col_offset > 0 && !scope.is_last_char(' ') {
                        ctx.col_offset += scope.write(" ");
                    }
                    let text = format!("@{}", n);
                    if ctx.col_offset == 0 {
                        ctx.col_offset += scope.write_with_ident(text);
                    } else {
                        ctx.col_offset += scope.write(text);
                    }
                }
                ANNOTATION_VALUE => format_annotation_value(scope.clone(), n, ctx),
                _ => {}
            },
            NodeOrToken::Token(t) => match t.kind() {
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }
}

fn format_annotation_value(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != ANNOTATION_VALUE {
        scope.write(&syntax.to_string());
        return;
    }
    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => {
                if n.kind() == VALUE {
                    match plain_value_to_tokens(syntax.clone()) {
                        Some(tokens) => {
                            let token_texts: Vec<&str> = tokens.iter().map(|v| v.text()).collect();
                            let text = token_texts.join("");
                            ctx.col_offset += scope.write(text);
                        }
                        None => {
                            format_value(scope.clone(), n, ctx);
                        }
                    }
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                PARENTHESES_START => {
                    if ctx.col_offset == 0 {
                        ctx.col_offset += scope.write_with_ident("(");
                    } else {
                        ctx.col_offset += scope.write("(");
                    }
                }
                PARENTHESES_END => {
                    if scope.is_last_char(',') {
                        scope.remove_last_char();
                    }
                    if ctx.col_offset > 0 {
                        ctx.col_offset += scope.write(")");
                    } else {
                        ctx.col_offset += scope.write_with_ident(")");
                    }
                }
                ERROR => format_error(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }
}

fn format_comment(scope: Scope, syntax: SyntaxToken, ctx: &mut Context) {
    let kind = syntax.kind();
    assert!(kind.is_comment());
    if kind == BLOCK_COMMENT {
        let text = syntax.text();
        if is_multiline(text) {
            if ctx.col_offset > 0 {
                scope.write("\n");
            }
            scope.write(ident_block_comment(text, &scope.ident()));
            scope.write("\n");
            ctx.col_offset = 0;
        } else {
            ctx.maybe_insert_space(&scope);
            ctx.col_offset += scope.write(text);
        }
    } else if kind == LINE_COMMENT {
        ctx.maybe_insert_space(&scope);
        let text = syntax.text();
        scope.write(text.trim());
        scope.write("\n");
        ctx.col_offset = 0;
    }
}

fn format_newline(scope: Scope, syntax: SyntaxToken, ctx: &mut Context) {
    assert!(syntax.kind() == NEWLINE);
    let text = syntax.text();
    let mut count = count_newlines(text);
    if ctx.col_offset == 0 {
        count -= 1;
    }
    scope.write("\n".repeat(count));
    ctx.col_offset = 0;
}

fn format_error(scope: Scope, syntax: SyntaxToken, ctx: &mut Context) {
    assert!(syntax.kind() == ERROR);
    ctx.col_offset += scope.write(syntax.text());
}

fn plain_value_to_tokens(syntax: SyntaxNode) -> Option<Vec<SyntaxToken>> {
    let mut tokens = vec![];
    for event in syntax.preorder_with_tokens() {
        if let WalkEvent::Enter(ele) = event {
            if let Some(t) = ele.as_token() {
                match t.kind() {
                    IDENT | FLOAT | BOOL | NULL | SINGLE_QUOTE | DOUBLE_QUOTE | BACKTICK_QUOTE
                    | INTEGER | INTEGER_BIN | INTEGER_HEX | INTEGER_OCT | COLON | COMMA
                    | BRACE_START | BRACE_END | BRACKET_START | BRACKET_END => {
                        tokens.push(t.clone());
                    }
                    NEWLINE | LINE_COMMENT => return None,
                    BLOCK_COMMENT => {
                        if is_multiline(t.text()) {
                            return None;
                        } else {
                            tokens.push(t.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Some(tokens)
}

fn is_multiline(text: &str) -> bool {
    text.contains('\n')
}

fn ident_block_comment(text: &str, ident: &str) -> String {
    let lines: Vec<String> = text
        .split('\n')
        .map(|v| format!("{}{}", ident, v.trim()))
        .collect();
    lines.join("\n")
}

fn count_newlines(text: &str) -> usize {
    text.chars().filter(|v| v.is_whitespace()).count()
}
