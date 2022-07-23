//! This module is used to format JSONA.
//!
//! The formatting can be done on documents that might
//! contain invalid syntax. In that case the invalid part is skipped.

mod options;
pub use options::*;

use std::{cell::RefCell, rc::Rc};

use crate::{
    parser,
    syntax::{SyntaxKind::*, SyntaxNode, SyntaxToken},
};

use rowan::{NodeOrToken, WalkEvent};

#[derive(Debug, Clone, Default)]
pub(crate) struct Scope {
    pub(crate) options: Rc<Options>,
    pub(crate) level: usize,
    pub(crate) formatted: Rc<RefCell<String>>,
    pub(crate) kind: ScopeKind,
    pub(crate) compact: bool,
}

impl Scope {
    pub(crate) fn enter(&self, kind: ScopeKind) -> Self {
        Self {
            options: self.options.clone(),
            level: self.level.saturating_add(1),
            formatted: self.formatted.clone(),
            kind,
            compact: self.compact,
        }
    }
    pub(crate) fn exit(&self) -> Self {
        Self {
            options: self.options.clone(),
            level: self.level.saturating_sub(1),
            formatted: self.formatted.clone(),
            kind: self.kind,
            compact: self.compact,
        }
    }
    pub(crate) fn write<T: AsRef<str>>(&self, text: T) -> usize {
        let text = text.as_ref();
        let len = text.len();
        self.formatted.borrow_mut().push_str(text);
        len
    }
    pub(crate) fn write_ident(&self) -> usize {
        let ident = self.ident_string();
        self.formatted.borrow_mut().push_str(&ident);
        ident.len()
    }
    pub(crate) fn output(&self) -> String {
        let trailing_newline = self.options.trailing_newline;
        let mut formatted = self.formatted.borrow().to_string();
        if formatted.ends_with('\n') {
            formatted.truncate(formatted.len() - 1);
        }
        if trailing_newline {
            formatted += "\n";
        }
        formatted
    }
    pub(crate) fn ident_string(&self) -> String {
        self.options.indent_string.repeat(self.level)
    }
    pub(crate) fn is_last_char(&self, c: char) -> bool {
        self.formatted
            .borrow()
            .chars()
            .last()
            .map(|v| v == c)
            .unwrap_or_default()
    }
    pub(crate) fn remove_last_char(&self) {
        self.formatted.borrow_mut().pop();
    }
    pub(crate) fn newline(&self) {
        if !self.is_last_char('\n') {
            self.write("\n");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeKind {
    Root,
    Array,
    Object,
}

impl Default for ScopeKind {
    fn default() -> Self {
        ScopeKind::Root
    }
}

#[derive(Debug, Clone)]
struct Context {
    col_offset: usize,
    value_commas: Vec<ValueComma>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueComma {
    ForceYes,
    ForceNo,
    Auto,
}

impl Context {
    fn space(&mut self, scope: &Scope) {
        if self.col_offset > 0 && !scope.is_last_char(' ') {
            self.write(scope, " ")
        }
    }

    fn comma(&mut self, scope: &Scope) {
        match self.value_commas.last() {
            Some(ValueComma::Auto) => {
                if !scope.compact && scope.options.trailing_comma {
                    self.write(scope, ",")
                }
            }
            Some(ValueComma::ForceYes) => {
                if scope.compact {
                    self.write(scope, ", ")
                } else {
                    self.write(scope, ",")
                }
            }
            _ => {}
        };
    }

    fn newline(&mut self, scope: &Scope) {
        if !scope.compact && self.col_offset > 0 {
            scope.write("\n");
            self.col_offset = 0;
        }
    }
    fn ident(&mut self, scope: &Scope) {
        if self.col_offset == 0 {
            self.col_offset += scope.write_ident();
        }
    }
    fn write<T: AsRef<str>>(&mut self, scope: &Scope, text: T) {
        self.col_offset += scope.write(text)
    }
}

/// Parses then formats a JSONA document, skipping ranges that contain syntax errors.
pub fn format(src: &str, options: Options) -> String {
    let p = parser::parse(src);
    let scope = Scope {
        options: Rc::new(options),
        ..Default::default()
    };
    let mut ctx = Context {
        col_offset: 0,
        value_commas: vec![],
    };
    format_value(scope.clone(), p.into_syntax(), &mut ctx);
    scope.output()
}

/// Formats a parsed JSONA syntax tree.
pub fn format_syntax(node: SyntaxNode, options: Options) -> String {
    let scope = Scope {
        options: Rc::new(options),
        ..Default::default()
    };
    let mut ctx = Context {
        col_offset: 0,
        value_commas: vec![],
    };
    format_value(scope.clone(), node, &mut ctx);
    scope.output()
}

fn format_value(mut scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != VALUE {
        scope.write(&syntax.to_string());
        return;
    }

    scope.compact = can_compact(syntax.clone());

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
    if scope.kind == ScopeKind::Array {
        ctx.newline(&scope);
        ctx.ident(&scope);
        ctx.write(&scope, &text)
    } else {
        ctx.write(&scope, &text)
    }
    if is_multiline(&text) {
        if let Some(offset) = text.split('\n').last().map(|v| v.len()) {
            ctx.col_offset = offset
        }
    }
    ctx.comma(&scope);
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
                    PROPERTY => {
                        if let Some(c) = ctx.value_commas.last_mut() {
                            if n.next_sibling().is_none() {
                                *c = ValueComma::Auto;
                            }
                        }
                        format_entry(scope.clone(), n, ctx);
                    }
                    ANNOTATIONS => format_annotations(scope.clone(), n, ctx),
                    _ => {}
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                BRACE_START => {
                    ctx.ident(&scope);
                    ctx.write(&scope, "{");
                    scope = scope.enter(ScopeKind::Object);
                    ctx.value_commas.push(ValueComma::ForceYes);
                }
                BRACE_END => {
                    scope = scope.exit();
                    ctx.value_commas.pop();
                    if !is_empty {
                        ctx.newline(&scope);
                    }
                    ctx.ident(&scope);
                    ctx.write(&scope, "}");
                }
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }
    ctx.comma(&scope);
}

fn format_entry(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != PROPERTY {
        scope.write(&syntax.to_string());
        return;
    }
    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                KEY => {
                    let text = n.to_string();
                    ctx.newline(&scope);
                    ctx.ident(&scope);
                    ctx.write(&scope, &text);
                }
                VALUE => format_value(scope.clone(), n, ctx),
                _ => {}
            },
            NodeOrToken::Token(t) => match t.kind() {
                COLON => ctx.write(&scope, ": "),
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
                        if let Some(c) = ctx.value_commas.last_mut() {
                            if n.next_sibling().is_none() {
                                *c = ValueComma::Auto;
                            }
                        }
                        format_value(scope.clone(), n, ctx);
                    }
                    ANNOTATIONS => format_annotations(scope.clone(), n, ctx),
                    _ => {}
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                BRACKET_START => {
                    ctx.ident(&scope);
                    ctx.write(&scope, "[");
                    scope = scope.enter(ScopeKind::Array);
                    ctx.value_commas.push(ValueComma::ForceYes);
                }
                BRACKET_END => {
                    scope = scope.exit();
                    ctx.value_commas.pop();
                    if !is_empty {
                        ctx.newline(&scope);
                    }
                    ctx.ident(&scope);
                    ctx.write(&scope, "]");
                }
                ERROR => format_error(scope.clone(), t, ctx),
                NEWLINE => format_newline(scope.clone(), t, ctx),
                k if k.is_comment() => format_comment(scope.clone(), t, ctx),
                _ => {}
            },
        }
    }

    ctx.comma(&scope);
}

fn format_annotations(scope: Scope, syntax: SyntaxNode, ctx: &mut Context) {
    if syntax.kind() != ANNOTATIONS {
        scope.write(&syntax.to_string());
        return;
    }

    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => {
                if n.kind() == ANNOTATION_PROPERTY {
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
    if syntax.kind() != ANNOTATION_PROPERTY {
        scope.write(&syntax.to_string());
        return;
    }

    for c in syntax.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                ANNOTATION_VALUE => format_annotation_value(scope.clone(), n, ctx),
                _ => {}
            },
            NodeOrToken::Token(t) => match t.kind() {
                ANNOATION_KEY => {
                    if ctx.col_offset > 0 && !scope.is_last_char(' ') {
                        ctx.col_offset += scope.write(" ");
                    }
                    ctx.ident(&scope);
                    ctx.write(&scope, t.text());
                }
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
                    format_value(scope.clone(), n, ctx);
                }
            }
            NodeOrToken::Token(t) => match t.kind() {
                PARENTHESES_START => {
                    ctx.ident(&scope);
                    ctx.write(&scope, "(");
                    ctx.value_commas.push(ValueComma::ForceNo);
                }
                PARENTHESES_END => {
                    ctx.value_commas.pop();
                    ctx.ident(&scope);
                    ctx.write(&scope, ")");
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
            scope.write(ident_block_comment(text, &scope.ident_string()));
            scope.write("\n");
            ctx.col_offset = 0;
        } else {
            ctx.space(&scope);
            ctx.write(&scope, text)
        }
    } else if kind == LINE_COMMENT {
        ctx.space(&scope);
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
    ctx.write(&scope, syntax.text())
}

fn can_compact(syntax: SyntaxNode) -> bool {
    let mut exist_newline = false;
    for event in syntax.preorder_with_tokens() {
        if let WalkEvent::Enter(ele) = event {
            if let Some(t) = ele.as_token() {
                if t.kind() == WHITESPACE {
                    continue;
                }
                if exist_newline {
                    return false;
                }
                match t.kind() {
                    BLOCK_COMMENT => {
                        if is_multiline(t.text()) {
                            return false;
                        }
                    }
                    NEWLINE => {
                        exist_newline = true;
                    }
                    _ => {}
                }
            }
        }
    }
    true
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
