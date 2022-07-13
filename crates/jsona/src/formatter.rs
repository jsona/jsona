use crate::{
    dom::{from_syntax::key_from_syntax, KeyOrIndex, Keys},
    parser,
    syntax::{SyntaxElement, SyntaxKind::*, SyntaxNode},
};

use rowan::{NodeOrToken, TextRange};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

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
        /// Automatically collapse annotations if they
        /// fit in one line.
        ///
        /// The annotations won't be collapsed if it
        /// contains a comment.
        pub annotations_auto_collapse: bool,
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

        /// Use CRLF line endings
        pub crlf: bool,
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
            annotations_auto_collapse: true,
            column_width: 80,
            indent_string: "  ".into(),
            trailing_comma: true,
            trailing_newline: true,
            crlf: false,
        }
    }
}

impl Options {
    pub(crate) fn newline(&self) -> &'static str {
        if self.crlf {
            "\r\n"
        } else {
            "\n"
        }
    }
}

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
    states: Rc<RefCell<HashMap<Keys, ScopeKeysState>>>,
    check_errors: Rc<RefCell<HashSet<Keys>>>,
}
#[derive(Debug, Clone, Default)]
struct ScopeKeysState {
    multiline: bool,
    errors: usize,
    lens: Vec<SegLen>,
    nums: usize,
}

#[derive(Debug, Clone)]
enum SegLen {
    Key(usize),
    Comment(usize),
    Value(usize),
}

impl Scope {
    fn new(options: Options, errors: Vec<TextRange>) -> Self {
        Self {
            options: Rc::new(options),
            errors: Rc::from(errors),
            keys: Keys::empty(),
            states: Default::default(),
            check_errors: Default::default(),
        }
    }

    fn spawn_child(&self, key: KeyOrIndex) -> Self {
        Self {
            options: self.options.clone(),
            errors: self.errors.clone(),
            keys: self.keys.clone().join(key),
            states: self.states.clone(),
            check_errors: Default::default(),
        }
    }

    fn contain_annotation_key(&self) -> bool {
        self.keys.iter().any(|v| v.is_annotation_key())
    }

    fn check_error(&self, range: TextRange) {
        if self.check_errors.borrow().contains(&self.keys) {
            return;
        }
        for error_range in self.errors.iter().copied() {
            if range.contains_range(error_range) {
                self.check_errors.borrow_mut().insert(self.keys.clone());
                for keys in self.keys.iter_keys() {
                    self.states.borrow_mut().entry(keys).or_default().errors += 1;
                }
            }
        }
    }

    fn is_error(&self) -> bool {
        if let Some(1) = self.states.borrow().get(&self.keys).map(|v| v.errors) {
            true
        } else {
            false
        }
    }

    fn state(&self) -> (bool, usize) {
        if let Some(state) = self.states.borrow_mut().get(&self.keys) {
            (state.multiline, state.nums)
        } else {
            (false, 0)
        }
    }

    fn inc_num(&self) {
        self.states
            .borrow_mut()
            .entry(self.keys.clone())
            .or_default()
            .nums += 1;
    }

    fn add_len(&self, type_len: SegLen) {
        self.states
            .borrow_mut()
            .entry(self.keys.clone())
            .or_default()
            .lens
            .push(type_len);
    }

    fn multiline(&self) {
        let multiline = self
            .states
            .borrow()
            .get(&self.keys)
            .map(|v| v.multiline)
            .unwrap_or_default();
        if multiline {
            return;
        }
        for keys in self.keys.iter_keys() {
            self.states.borrow_mut().entry(keys).or_default().multiline = true;
        }
    }
}

fn format_impl(node: SyntaxNode, scope: Scope) -> String {
    assert!(node.kind() == VALUE);
    let mut formatted = format_root(node, scope.clone());

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

fn format_root(node: SyntaxNode, scope: Scope) -> String {
    assert!(node.kind() == VALUE);
    preflight_value(node.clone(), scope.clone());
    for (k, v) in scope.states.borrow().iter() {
        println!("{} {:?}", k, v);
    }
    format_value(node.clone(), scope.clone())
}

fn format_value(node: SyntaxNode, scope: Scope) -> String {
    todo!()
}

fn preflight_value(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == VALUE);
    scope.check_error(node.text_range());
    for c in node.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                OBJECT => preflight_object(n, scope.clone()),
                ARRAY => preflight_array(n, scope.clone()),
                ANNOTATIONS => preflight_annotations(n, scope.clone()),
                _ => {}
            },
            NodeOrToken::Token(t) => {
                let kind = t.kind();
                if kind.is_scalar() {
                    let value = t.to_string();
                    if contain_newline(&value) {
                        scope.multiline();
                    } else {
                        scope.add_len(SegLen::Value(value.len()));
                    }
                } else {
                    preflight_comment(t.into(), scope.clone());
                }
            }
        }
    }
}

fn preflight_object(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == OBJECT);
    for c in node.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                ENTRY => preflight_entry(n, scope.clone()),
                ANNOTATIONS => preflight_annotations(n, scope.clone()),
                _ => {}
            },
            NodeOrToken::Token(_t) => {}
        }
    }
}

fn preflight_array(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == ARRAY);
    let mut index: usize = 0;
    for c in node.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                VALUE => {
                    scope.inc_num();
                    let key: KeyOrIndex = index.into();
                    let scope = scope.spawn_child(key);
                    preflight_value(n, scope);
                    index += 1;
                }
                ANNOTATIONS => preflight_annotations(n, scope.clone()),
                _ => {}
            },
            NodeOrToken::Token(_t) => {}
        }
    }
}

fn preflight_annotations(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == ANNOTATIONS);
    for c in node.children_with_tokens() {
        match c {
            NodeOrToken::Node(n) => match n.kind() {
                ANNOTATION_ENTRY => preflight_annotation_entry(n, scope.clone()),
                _ => {
                    preflight_comment(n.into(), scope.clone());
                }
            },
            NodeOrToken::Token(t) => {
                preflight_comment(t.into(), scope.clone());
            }
        }
    }
}

fn preflight_annotation_entry(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == ANNOTATION_ENTRY);
    if scope.contain_annotation_key() {
        return;
    }
    if let Some(key_syntax) = node.children().find(|v| v.kind() == KEY) {
        let key = key_from_syntax(key_syntax.into());
        let key = KeyOrIndex::AnnotationKey(key);
        let scope = scope.spawn_child(key);
        for c in node.children_with_tokens() {
            match c {
                NodeOrToken::Node(n) => match n.kind() {
                    KEY => preflight_key(n, scope.clone()),
                    VALUE => preflight_value(n, scope.clone()),
                    _ => {
                        preflight_comment(n.into(), scope.clone());
                    }
                },
                NodeOrToken::Token(t) => {
                    preflight_comment(t.into(), scope.clone());
                }
            }
        }
    }
}

fn preflight_entry(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == ENTRY);
    if let Some(key_syntax) = node.children().find(|v| v.kind() == KEY) {
        scope.inc_num();
        let key = key_from_syntax(key_syntax.into());
        let key = KeyOrIndex::Key(key);
        let scope = scope.spawn_child(key);
        for c in node.children_with_tokens() {
            match c {
                NodeOrToken::Node(n) => match n.kind() {
                    KEY => preflight_key(n, scope.clone()),
                    VALUE => preflight_value(n, scope.clone()),
                    _ => {
                        preflight_comment(n.into(), scope.clone());
                    }
                },
                NodeOrToken::Token(t) => {
                    preflight_comment(t.into(), scope.clone());
                }
            }
        }
    }
}

fn preflight_key(node: SyntaxNode, scope: Scope) {
    assert!(node.kind() == KEY);
    for c in node.children_with_tokens() {
        match c.kind() {
            IDENT => {
                scope.add_len(SegLen::Key(c.to_string().len()));
            }
            _ => {
                preflight_comment(c, scope.clone());
            }
        }
    }
}

fn preflight_comment(syntax: SyntaxElement, scope: Scope) {
    match syntax.kind() {
        BLOCK_COMMENT => {
            let value = syntax.to_string();
            if contain_newline(&value) {
                scope.multiline();
            } else {
                scope.add_len(SegLen::Comment(value.len()));
            }
        }
        LINE_COMMENT => {
            scope.multiline();
        }
        _ => {}
    }
}

fn contain_newline(s: &str) -> bool {
    s.chars().any(|c| c == '\n')
}
