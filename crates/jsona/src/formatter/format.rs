use super::Options;

use crate::{
    dom::{KeyOrIndex, Keys},
    parser,
    syntax::{SyntaxElement, SyntaxKind::*, SyntaxNode},
};

use rowan::{NodeOrToken, TextRange};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Parses then formats a JSONA document, skipping ranges that contain syntax errors.
pub fn format(src: &str, options: Options) -> String {
    let p = parser::parse(src);

    let scope = Scope::new(options, p.errors.iter().map(|err| err.range).collect());

    format_impl(p.into_syntax(), scope)
}

#[derive(Debug, Clone)]
struct Scope {
    options: Rc<Options>,
    errors: Rc<[TextRange]>,
    keys: Keys,
    multilines: Rc<RefCell<HashMap<Keys, bool>>>,
    textsizes: Rc<RefCell<HashMap<Keys, usize>>>,
    headsize: usize,
}

impl Scope {
    fn new(options: Options, errors: Vec<TextRange>) -> Self {
        Self {
            options: Rc::new(options),
            errors: Rc::from(errors),
            keys: Keys::empty(),
            multilines: Default::default(),
            textsizes: Default::default(),
            headsize: 0,
        }
    }

    fn spawn_child(&self, key: KeyOrIndex, headsize: usize) -> Self {
        Self {
            options: self.options.clone(),
            errors: self.errors.clone(),
            keys: self.keys.clone().join(key),
            multilines: self.multilines.clone(),
            textsizes: self.textsizes.clone(),
            headsize,
        }
    }

    fn detect_error(&self, syntax: &SyntaxElement) -> bool {
        if self.error_at(syntax.text_range()) {
            if contain_newline(&syntax.to_string()) {
                self.multiline();
            }
            true
        } else {
            false
        }
    }

    fn error_at(&self, range: TextRange) -> bool {
        for error_range in self.errors.iter().copied() {
            if overlaps(range, error_range) {
                return true;
            }
        }
        false
    }

    fn textsize(&self, value: usize) {
        self.textsizes.borrow_mut().insert(self.keys.clone(), value);
    }

    fn multiline(&self) {
        if let Some(false) = self.multilines.borrow().get(&self.keys) {
            return;
        }
        for keys in self.keys.iter_keys() {
            self.multilines.borrow_mut().insert(keys, true);
        }
    }
}

fn format_impl(node: SyntaxNode, scope: Scope) -> String {
    assert!(node.kind() == VALUE);
    let mut formatted = format_value(node, scope.clone());

    if formatted.ends_with("\r\n") {
        formatted.truncate(formatted.len() - 2);
    } else if formatted.ends_with('\n') {
        formatted.truncate(formatted.len() - 1);
    }

    if scope.options.trailing_newline {
        formatted += scope.options.newline();
    }

    formatted
}

fn format_value(node: SyntaxNode, scope: Scope) -> String {
    preflight_value(node.clone(), scope);
    assert!(node.kind() == VALUE);
    todo!()
}

fn preflight_value(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == VALUE);
    let mut textsize = 0;
    for c in node.children_with_tokens() {
        if scope.detect_error(&c) {
            continue;
        }
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                OBJECT => preflight_object_value(n, scope.clone()),
                ARRAY => preflight_array_value(n, scope.clone()),
                ANNOTATIONS => preflight_annotations_value(n, scope.clone()),
                _ => {}
            },
            NodeOrToken::Token(t) => match t.kind() {
                COMMENT_LINE | NEWLINE => {
                    scope.multiline();
                }
                COMMA => {
                    textsize += 1;
                }
                k if k.is_scalar() || k == COMMENT_BLOCK => {
                    let value = t.to_string();
                    if contain_newline(&value) {
                        scope.multiline();
                    } else {
                        textsize += value.len();
                    }
                }
                _ => {}
            },
        }
    }
    scope.textsize(textsize);
}

fn preflight_object_value(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == OBJECT);
    for c in node.children_with_tokens() {
        if scope.error_at(c.text_range()) {
            if contain_newline(&c.to_string()) {
                scope.multiline();
            }
            continue;
        }
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                ENTRY => {}
                ANNOTATIONS => {
                    preflight_annotations_value(n, scope.clone());
                }
                _ => {}
            },
            NodeOrToken::Token(_t) => {}
        }
    }
}

fn preflight_array_value(_node: SyntaxNode, _scope: Scope) {}

fn preflight_annotations_value(_node: SyntaxNode, _scope: Scope) {}

fn overlaps(range: TextRange, other: TextRange) -> bool {
    range.contains_range(other)
        || other.contains_range(range)
        || range.contains(other.start())
        || range.contains(other.end())
        || other.contains(range.start())
        || other.contains(range.end())
}

fn contain_newline(s: &str) -> bool {
    s.chars().any(|c| c == '\n')
}
