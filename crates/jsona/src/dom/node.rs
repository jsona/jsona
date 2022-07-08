use super::error::Error;
use super::key::Key;
use crate::private::Sealed;
use crate::syntax::SyntaxElement;
use crate::util::escape::unescape;
use crate::util::shared::Shared;

use once_cell::unsync::OnceCell;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::Arc;

macro_rules! wrap_node {
    (
    $(#[$attrs:meta])*
    $vis:vis struct $name:ident {
        inner: $inner:ident
    }
    ) => {
        $(#[$attrs])*
        $vis struct $name {
            pub(crate) inner: Arc<$inner>,
        }

        impl $crate::private::Sealed for $name {}
        impl $crate::dom::node::DomNode for $name {
            fn syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.syntax.as_ref()
            }

            fn errors(&self) -> &$crate::util::shared::Shared<Vec<$crate::dom::error::Error>> {
                &self.inner.errors
            }

            fn annos(&self) -> &$crate::util::shared::Shared<$crate::dom::node::Entries> {
                &self.inner.annos
            }

            fn validate_node(&self) -> Result<(), &$crate::util::shared::Shared<Vec<$crate::dom::error::Error>>> {
                self.validate_impl()
            }
        }

        impl $inner {
            #[allow(dead_code)]
            pub(crate) fn wrap(self) -> $name {
                self.into()
            }
        }

        impl From<$inner> for $name {
            fn from(inner: $inner) -> $name {
                $name {
                    inner: Arc::new(inner)
                }
            }
        }
    };
}

pub trait DomNode: Sized + Sealed {
    fn syntax(&self) -> Option<&SyntaxElement>;
    fn errors(&self) -> &Shared<Vec<Error>>;
    fn annos(&self) -> &Shared<Entries>;
    fn validate_node(&self) -> Result<(), &Shared<Vec<Error>>>;
    fn is_valid_node(&self) -> bool {
        self.validate_node().is_ok()
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Null(Null),
    Bool(Bool),
    Integer(Integer),
    Float(Float),
    Str(Str),
    Array(Array),
    Object(Object),
    Invalid(Invalid),
}

impl Sealed for Node {}
impl DomNode for Node {
    fn syntax(&self) -> Option<&SyntaxElement> {
        match self {
            Node::Null(n) => n.syntax(),
            Node::Bool(n) => n.syntax(),
            Node::Integer(n) => n.syntax(),
            Node::Float(n) => n.syntax(),
            Node::Str(n) => n.syntax(),
            Node::Array(n) => n.syntax(),
            Node::Object(n) => n.syntax(),
            Node::Invalid(n) => n.syntax(),
        }
    }

    fn errors(&self) -> &Shared<Vec<Error>> {
        match self {
            Node::Null(n) => n.errors(),
            Node::Bool(n) => n.errors(),
            Node::Integer(n) => n.errors(),
            Node::Float(n) => n.errors(),
            Node::Str(n) => n.errors(),
            Node::Array(n) => n.errors(),
            Node::Object(n) => n.errors(),
            Node::Invalid(n) => n.errors(),
        }
    }

    fn annos(&self) -> &Shared<Entries> {
        match self {
            Node::Null(n) => n.annos(),
            Node::Bool(n) => n.annos(),
            Node::Integer(n) => n.annos(),
            Node::Float(n) => n.annos(),
            Node::Str(n) => n.annos(),
            Node::Array(n) => n.annos(),
            Node::Object(n) => n.annos(),
            Node::Invalid(n) => n.annos(),
        }
    }

    fn validate_node(&self) -> Result<(), &Shared<Vec<Error>>> {
        match self {
            Node::Null(n) => n.validate_node(),
            Node::Bool(n) => n.validate_node(),
            Node::Integer(n) => n.validate_node(),
            Node::Float(n) => n.validate_node(),
            Node::Str(n) => n.validate_node(),
            Node::Array(n) => n.validate_node(),
            Node::Object(n) => n.validate_node(),
            Node::Invalid(n) => n.validate_node(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct NullInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Null { inner: NullInner }
}

impl Null {
    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug)]
pub(crate) struct BoolInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
    pub(crate) value: OnceCell<bool>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Bool { inner: BoolInner }
}

impl Bool {
    /// A boolean value.
    pub fn value(&self) -> bool {
        *self.inner.value.get_or_init(|| {
            self.syntax()
                .and_then(|s| s.as_token())
                .and_then(|s| s.text().parse().ok())
                .unwrap_or_default()
        })
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug)]
pub(crate) struct IntegerInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
    pub(crate) repr: IntegerRepr,
    pub(crate) value: OnceCell<IntegerValue>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Integer { inner: IntegerInner }
}

impl Integer {
    /// An integer value.
    pub fn value(&self) -> IntegerValue {
        *self.inner.value.get_or_init(|| {
            if let Some(s) = self.syntax().and_then(|s| s.as_token()) {
                let int_text = s.text().replace('_', "");

                match self.inner.repr {
                    IntegerRepr::Dec => {
                        if s.text().starts_with('-') {
                            IntegerValue::Negative(int_text.parse().unwrap_or_default())
                        } else {
                            IntegerValue::Positive(int_text.parse().unwrap_or_default())
                        }
                    }
                    IntegerRepr::Bin => IntegerValue::Positive(
                        u64::from_str_radix(int_text.trim_start_matches("0b"), 2)
                            .unwrap_or_default(),
                    ),
                    IntegerRepr::Oct => IntegerValue::Positive(
                        u64::from_str_radix(int_text.trim_start_matches("0o"), 8)
                            .unwrap_or_default(),
                    ),
                    IntegerRepr::Hex => IntegerValue::Positive(
                        u64::from_str_radix(int_text.trim_start_matches("0x"), 16)
                            .unwrap_or_default(),
                    ),
                }
            } else {
                IntegerValue::Positive(0)
            }
        })
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum IntegerRepr {
    Dec,
    Bin,
    Oct,
    Hex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerValue {
    Negative(i64),
    Positive(u64),
}

impl IntegerValue {
    /// Returns `true` if the integer value is [`Negative`].
    ///
    /// [`Negative`]: IntegerValue::Negative
    pub fn is_negative(&self) -> bool {
        matches!(self, Self::Negative(..))
    }

    /// Returns `true` if the integer value is [`Positive`].
    ///
    /// [`Positive`]: IntegerValue::Positive
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Positive(..))
    }

    pub fn as_negative(&self) -> Option<i64> {
        if let Self::Negative(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_positive(&self) -> Option<u64> {
        if let Self::Positive(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

impl core::fmt::Display for IntegerValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegerValue::Negative(v) => v.fmt(f),
            IntegerValue::Positive(v) => v.fmt(f),
        }
    }
}

#[derive(Debug)]
pub(crate) struct FloatInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
    pub(crate) value: OnceCell<f64>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Float { inner: FloatInner }
}

impl Float {
    /// A float value.
    pub fn value(&self) -> f64 {
        *self.inner.value.get_or_init(|| {
            if let Some(text) = self.syntax().and_then(|s| s.as_token()).map(|s| s.text()) {
                text.replace('_', "").parse().unwrap()
            } else {
                0_f64
            }
        })
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        let _ = self.value();
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug)]
pub(crate) struct StrInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
    pub(crate) repr: StrRepr,
    pub(crate) value: OnceCell<String>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Str { inner: StrInner }
}

impl Str {
    /// An unescaped value of the string.
    pub fn value(&self) -> &str {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .map(|s| match self.inner.repr {
                    StrRepr::Double => {
                        let string = s.as_token().unwrap().text();
                        let string = string.strip_prefix('"').unwrap_or(string);
                        let string = string.strip_suffix('"').unwrap_or(string);
                        match unescape(string) {
                            Ok(s) => s,
                            Err(_) => {
                                self.inner.errors.update(|errors| {
                                    errors.push(Error::InvalidEscapeSequence { string: s.clone() })
                                });
                                String::new()
                            }
                        }
                    }
                    StrRepr::Single => {
                        let string = s.as_token().unwrap().text();
                        let string = string.strip_prefix('\'').unwrap_or(string);
                        let string = string.strip_suffix('\'').unwrap_or(string);
                        match unescape(string) {
                            Ok(s) => s,
                            Err(_) => {
                                self.inner.errors.update(|errors| {
                                    errors.push(Error::InvalidEscapeSequence { string: s.clone() })
                                });
                                String::new()
                            }
                        }
                    }
                    StrRepr::Backtick => {
                        let string = s.as_token().unwrap().text();
                        let string = string.strip_prefix(r#"`"#).unwrap_or(string);
                        let string = match string.strip_prefix("\r\n") {
                            Some(s) => s,
                            None => string.strip_prefix('\n').unwrap_or(string),
                        };
                        let string = string.strip_suffix(r#"`"#).unwrap_or(string);
                        string.to_string()
                    }
                })
                .unwrap_or_default()
        })
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        let _ = self.value();
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum StrRepr {
    Single,
    Double,
    Backtick,
}

#[derive(Debug)]
pub(crate) struct ArrayInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
    pub(crate) kind: ArrayKind,
    pub(crate) items: Shared<Vec<Node>>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Array { inner: ArrayInner }
}

impl Array {
    pub fn items(&self) -> &Shared<Vec<Node>> {
        &self.inner.items
    }

    pub fn kind(&self) -> ArrayKind {
        self.inner.kind
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayKind {
    Multiline,
    Inline,
}

impl ArrayKind {
    /// Returns `true` if the array kind is [`Multiline`].
    ///
    /// [`Multiline`]: ArrayKind::Multiline
    pub fn is_tables(&self) -> bool {
        matches!(self, Self::Multiline)
    }

    /// Returns `true` if the array kind is [`Inline`].
    ///
    /// [`Inline`]: ArrayKind::Inline
    pub fn is_inline(&self) -> bool {
        matches!(self, Self::Inline)
    }
}

#[derive(Debug)]
pub(crate) struct ObjectInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
    pub(crate) kind: ObjectKind,
    pub(crate) entries: Shared<Entries>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Object { inner: ObjectInner }
}

impl Object {
    pub fn get(&self, key: impl Into<Key>) -> Option<Node> {
        let key = key.into();
        let entries = self.inner.entries.read();
        entries.lookup.get(&key).cloned()
    }

    pub fn entries(&self) -> &Shared<Entries> {
        &self.inner.entries
    }

    pub fn kind(&self) -> ObjectKind {
        self.inner.kind
    }

    /// Add an entry and also collect errors on conflicts.
    pub(crate) fn add_entry(&self, key: Key, node: Node) {
        self.inner.entries.update(|entries| {
            if let Some((existing_key, value)) = entries.lookup.get_key_value(&key) {
                self.inner.errors.update(|errors| {
                    errors.push(Error::ConflictingKeys {
                        key: key.clone(),
                        other: existing_key.clone(),
                    })
                });
            }

            entries.add(key, node);
        });
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Multiline,
    Inline,
}

#[derive(Debug)]
pub(crate) struct InvalidInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Shared<Entries>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Invalid { inner: InvalidInner }
}

impl Invalid {
    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Entries {
    pub(crate) lookup: HashMap<Key, Node>,
    pub(crate) all: Vec<(Key, Node)>,
}

impl Entries {
    pub fn len(&self) -> usize {
        self.all.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Key, Node)> {
        self.all.iter()
    }

    pub(crate) fn add(&mut self, key: Key, node: Node) {
        self.lookup.insert(key.clone(), node.clone());
        self.all.push((key, node));
    }
}

impl FromIterator<(Key, Node)> for Entries {
    fn from_iter<T: IntoIterator<Item = (Key, Node)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let size = iter.size_hint().0;

        let mut lookup = HashMap::with_capacity(size);
        let mut all = Vec::with_capacity(size);

        for (k, n) in iter {
            lookup.insert(k.clone(), n.clone());
            all.push((k, n));
        }

        Self { lookup, all }
    }
}
