use super::error::{Error, ParseError, QueryError};
use super::keys::{KeyOrIndex, Keys};
use super::visitor::{VisitControl, Visitor};
use crate::parser;
use crate::private::Sealed;
use crate::syntax::SyntaxElement;
use crate::util::shared::Shared;
use crate::util::{quote, unquote, QuoteType};

use once_cell::unsync::OnceCell;
use rowan::{NodeOrToken, TextRange};
use serde_json::Number as JsonNumber;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::str::FromStr;
use std::string::String as StdString;
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
            fn node_syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.node_syntax.as_ref()
            }

            fn syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.syntax.as_ref()
            }

            fn errors(&self) -> &$crate::util::shared::Shared<Vec<$crate::dom::error::Error>> {
                &self.inner.errors
            }

            fn annotations(&self) -> Option<&$crate::dom::node::Annotations> {
                self.inner.annotations.as_ref()
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
    fn node_syntax(&self) -> Option<&SyntaxElement>;
    fn syntax(&self) -> Option<&SyntaxElement>;
    fn errors(&self) -> &Shared<Vec<Error>>;
    fn annotations(&self) -> Option<&Annotations>;
    fn validate_node(&self) -> Result<(), &Shared<Vec<Error>>>;
    fn is_valid_node(&self) -> bool {
        self.validate_node().is_ok()
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Null(Null),
    Bool(Bool),
    Number(Number),
    String(String),
    Array(Array),
    Object(Object),
}

macro_rules! impl_dom_node_for_node {
    (
        $(
          $elm:ident,
        )*
    ) => {
impl DomNode for Node {
    fn node_syntax(&self) -> Option<&SyntaxElement> {
        match self {
            $(
            Node::$elm(v) => v.node_syntax(),
            )*
        }
    }

    fn syntax(&self) -> Option<&SyntaxElement> {
        match self {
            $(
            Node::$elm(v) => v.syntax(),
            )*
        }
    }

    fn errors(&self) -> &Shared<Vec<Error>> {
        match self {
            $(
            Node::$elm(v) => v.errors(),
            )*
        }
    }

    fn annotations(&self) -> Option<&Annotations> {
        match self {
            $(
            Node::$elm(v) => v.annotations(),
            )*
        }
    }

    fn validate_node(&self) -> Result<(), &Shared<Vec<Error>>> {
        match self {
            $(
            Node::$elm(v) => v.validate_node(),
            )*
        }
    }
}
    };
}

impl Sealed for Node {}

impl_dom_node_for_node!(Null, Number, String, Bool, Array, Object,);

impl Node {
    pub fn path(&self, keys: &Keys) -> Option<Node> {
        let mut node = self.clone();
        for key in keys.iter() {
            node = node.get(key)?;
        }
        Some(node)
    }

    pub fn get(&self, key: &KeyOrIndex) -> Option<Node> {
        match key {
            KeyOrIndex::Index(i) => self.as_array().and_then(|v| v.get(*i)),
            KeyOrIndex::Key(k) => {
                if k.is_property() {
                    self.as_object().and_then(|v| v.get(k))
                } else {
                    self.annotations().and_then(|v| v.get(k))
                }
            }
            _ => None,
        }
    }

    pub fn try_get(&self, key: &KeyOrIndex) -> Result<Node, QueryError> {
        self.get(key).ok_or(QueryError::NotFound)
    }

    pub fn validate(&self) -> Result<(), impl Iterator<Item = Error> + core::fmt::Debug> {
        let mut errors = Vec::new();
        self.validate_all_impl(&mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into_iter())
        }
    }

    pub fn text_range(&self) -> Option<TextRange> {
        self.syntax().map(|v| v.text_range())
    }

    pub fn node_text_range(&self) -> Option<TextRange> {
        self.node_syntax().map(|v| v.text_range())
    }

    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Node::Null(_) | Node::Bool(_) | Node::Number(_) | Node::String(_)
        )
    }

    pub fn matches_all(
        &self,
        keys: Keys,
        match_children: bool,
    ) -> Result<impl Iterator<Item = (Keys, Node)> + ExactSizeIterator, Error> {
        let all: Vec<(Keys, Node)> = Visitor::new(self, &(), |_, _, _| VisitControl::AddIter)
            .into_iter()
            .collect();
        let mut output = vec![];
        for (k, v) in all {
            if keys.is_match(&k, match_children) {
                output.push((k, v));
            }
        }
        Ok(output.into_iter())
    }

    pub fn scalar_text(&self) -> Option<StdString> {
        match self {
            Node::Null(v) => {
                if v.is_valid_node() {
                    Some("null".to_string())
                } else {
                    None
                }
            }
            Node::Bool(v) => {
                let text = match self.syntax() {
                    Some(syntax) => syntax.to_string(),
                    None => v.value().to_string(),
                };
                Some(text)
            }
            Node::Number(v) => {
                let text = match self.syntax() {
                    Some(syntax) => syntax.to_string(),
                    None => v.value().to_string(),
                };
                Some(text)
            }
            Node::String(v) => {
                let text = match self.syntax() {
                    Some(syntax) => syntax.to_string(),
                    None => quote(v.value(), true),
                };
                Some(text)
            }
            Node::Array(_) | Node::Object(_) => None,
        }
    }

    fn validate_all_impl(&self, errors: &mut Vec<Error>) {
        match self {
            Node::Object(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }

                let items = v.inner.properties.read();
                for (k, entry) in items.as_ref().all.iter() {
                    if let Err(errs) = k.validate_node() {
                        errors.extend(errs.read().as_ref().iter().cloned())
                    }
                    entry.validate_all_impl(errors);
                }
            }
            Node::Array(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }

                let items = v.inner.items.read();
                for item in &**items.as_ref() {
                    if let Err(errs) = item.validate_node() {
                        errors.extend(errs.read().as_ref().iter().cloned())
                    }
                }
            }
            Node::Bool(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
            Node::String(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
            Node::Number(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
            Node::Null(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
        }
        if let Some(v) = self.annotations() {
            if let Err(errs) = v.validate_node() {
                errors.extend(errs.read().as_ref().iter().cloned())
            }

            let items = v.inner.members.read();
            for (k, node) in items.as_ref().all.iter() {
                if let Err(errs) = k.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
                node.validate_all_impl(errors);
            }
        }
    }
}

macro_rules! define_value_fns {
    ($elm:ident, $t:ty, $is_fn:ident, $as_fn:ident, $try_get_as_fn:ident) => {
        pub fn $is_fn(&self) -> bool {
            matches!(self, Self::$elm(..))
        }

        pub fn $as_fn(&self) -> Option<&$t> {
            if let Self::$elm(v) = self {
                Some(v)
            } else {
                None
            }
        }

        pub fn $try_get_as_fn(&self, key: &KeyOrIndex) -> Result<Option<$t>, QueryError> {
            match self.get(key) {
                None => Ok(None),
                Some(v) => {
                    if let Node::$elm(v) = v {
                        Ok(Some(v))
                    } else {
                        Err(QueryError::MismatchType)
                    }
                }
            }
        }
    };
}

impl Node {
    define_value_fns!(Null, Null, is_null, as_null, try_get_as_null);
    define_value_fns!(Bool, Bool, is_bool, as_bool, try_get_as_bool);
    define_value_fns!(Number, Number, is_number, as_number, try_get_as_number);
    define_value_fns!(String, String, is_string, as_string, try_get_as_string);
    define_value_fns!(Object, Object, is_object, as_object, try_get_as_object);
    define_value_fns!(Array, Array, is_array, as_array, try_get_as_array);
}

macro_rules! value_from {
    (
        $(
          $elm:ident,
        )*
    ) => {
    $(
    impl From<$elm> for Node {
        fn from(v: $elm) -> Self {
            Self::$elm(v)
        }
    }
    )*
    };
}

value_from!(Null, Number, String, Bool, Array, Object,);

impl FromStr for Node {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse = parser::parse(s);
        if !parse.errors.is_empty() {
            return Err(ParseError::InvalidSyntax {
                errors: parse.errors,
            });
        }
        let root = parse.into_dom();
        if let Err(errors) = root.validate() {
            return Err(ParseError::InvalidDom {
                errors: errors.collect(),
            });
        }
        Ok(root)
    }
}

#[derive(Debug, Default)]
pub(crate) struct NullInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
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
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
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
pub(crate) struct NumberInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) repr: NumberRepr,
    pub(crate) value: OnceCell<JsonNumber>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Number { inner: NumberInner }
}

impl Number {
    /// An nubmer value.
    pub fn value(&self) -> &JsonNumber {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .map(|s| {
                    let text = s.as_token().unwrap().text().replace('_', "");

                    match self.inner.repr {
                        NumberRepr::Dec => {
                            if text.starts_with('-') {
                                JsonNumber::from(text.parse::<i64>().unwrap_or_default())
                            } else {
                                JsonNumber::from(text.parse::<u64>().unwrap_or_default())
                            }
                        }
                        NumberRepr::Bin => JsonNumber::from(
                            u64::from_str_radix(text.trim_start_matches("0b"), 2)
                                .unwrap_or_default(),
                        ),
                        NumberRepr::Oct => JsonNumber::from(
                            u64::from_str_radix(text.trim_start_matches("0o"), 8)
                                .unwrap_or_default(),
                        ),
                        NumberRepr::Hex => JsonNumber::from(
                            u64::from_str_radix(text.trim_start_matches("0x"), 16)
                                .unwrap_or_default(),
                        ),
                        NumberRepr::Float => text
                            .parse::<f64>()
                            .ok()
                            .and_then(JsonNumber::from_f64)
                            .unwrap_or_else(|| JsonNumber::from_f64(0.0).unwrap()),
                    }
                })
                .unwrap_or_else(|| JsonNumber::from(0))
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
pub enum NumberRepr {
    Dec,
    Bin,
    Oct,
    Hex,
    Float,
}

#[derive(Debug)]
pub(crate) struct StringInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) repr: StringRepr,
    pub(crate) value: OnceCell<StdString>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct String { inner: StringInner }
}

impl String {
    /// An unescaped value of the string.
    pub fn value(&self) -> &str {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .map(|s| {
                    let quote_type = match self.inner.repr {
                        StringRepr::Double => QuoteType::Double,
                        StringRepr::Single => QuoteType::Single,
                        StringRepr::Backtick => QuoteType::Backtick,
                    };

                    let string = s.as_token().unwrap().text();
                    match unquote(string, quote_type) {
                        Ok(s) => s,
                        Err(_) => {
                            self.inner.errors.update(|errors| {
                                errors.push(Error::InvalidEscapeSequence { string: s.clone() })
                            });
                            StdString::new()
                        }
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
pub enum StringRepr {
    Single,
    Double,
    Backtick,
}

#[derive(Debug)]
pub(crate) struct ArrayInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) items: Shared<Vec<Node>>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Array { inner: ArrayInner }
}

impl Array {
    pub fn get(&self, idx: usize) -> Option<Node> {
        let items = self.inner.items.read();
        items.get(idx).cloned()
    }

    pub fn value(&self) -> &Shared<Vec<Node>> {
        &self.inner.items
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
pub(crate) struct ObjectInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) properties: Shared<Map>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Object { inner: ObjectInner }
}

impl Object {
    pub fn get(&self, key: &Key) -> Option<Node> {
        let props = self.inner.properties.read();
        props.lookup.get(key).cloned()
    }

    pub fn value(&self) -> &Shared<Map> {
        &self.inner.properties
    }

    pub fn properties_keys(&self) -> Vec<StdString> {
        self.value()
            .read()
            .iter()
            .map(|(k, _)| k.to_string())
            .collect()
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
pub(crate) struct KeyInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) value: OnceCell<StdString>,
    pub(crate) kind: KeyKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Property,
    Annotation,
}

#[derive(Debug, Clone)]
pub struct Key {
    inner: Arc<KeyInner>,
}

impl From<KeyInner> for Key {
    fn from(inner: KeyInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl Key {
    pub fn property<T: Into<StdString>>(key: T) -> Self {
        KeyInner {
            errors: Default::default(),
            syntax: None,
            value: OnceCell::from(key.into()),
            kind: KeyKind::Property,
        }
        .into()
    }

    pub fn annotation<T: Into<StdString>>(key: T) -> Self {
        KeyInner {
            errors: Default::default(),
            syntax: None,
            value: OnceCell::from(key.into()),
            kind: KeyKind::Annotation,
        }
        .into()
    }

    /// An unescaped value of the key.
    pub fn value(&self) -> &str {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .and_then(NodeOrToken::as_token)
                .map(|s| {
                    if self.is_annotation() {
                        return s.to_string();
                    }
                    let quote_type = match s.text().chars().next() {
                        Some('\'') => QuoteType::Single,
                        Some('"') => QuoteType::Double,
                        Some('`') => QuoteType::Backtick,
                        _ => QuoteType::None,
                    };

                    let string = s.text();
                    match unquote(string, quote_type) {
                        Ok(s) => s,
                        Err(_) => {
                            self.inner.errors.update(|errors| {
                                errors.push(Error::InvalidEscapeSequence {
                                    string: s.clone().into(),
                                })
                            });
                            StdString::new()
                        }
                    }
                })
                .unwrap_or_default()
        })
    }

    pub fn is_property(&self) -> bool {
        self.inner.kind == KeyKind::Property
    }

    pub fn is_annotation(&self) -> bool {
        self.inner.kind == KeyKind::Annotation
    }

    pub fn text_range(&self) -> Option<TextRange> {
        self.syntax().map(|v| v.text_range())
    }

    pub fn to_raw_string(&self) -> StdString {
        match self.syntax() {
            Some(v) => v.to_string(),
            None => self.value().to_string(),
        }
    }
}

impl Sealed for Key {}
impl DomNode for Key {
    fn node_syntax(&self) -> Option<&SyntaxElement> {
        self.inner.syntax.as_ref()
    }

    fn syntax(&self) -> Option<&SyntaxElement> {
        self.inner.syntax.as_ref()
    }

    fn errors(&self) -> &Shared<Vec<Error>> {
        &self.inner.errors
    }

    fn annotations(&self) -> Option<&Annotations> {
        None
    }

    fn validate_node(&self) -> Result<(), &Shared<Vec<Error>>> {
        let _ = self.value();
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

impl AsRef<str> for Key {
    fn as_ref(&self) -> &str {
        self.value()
    }
}

impl core::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_annotation() {
            self.value().fmt(f)
        } else {
            quote(self.value(), false).fmt(f)
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        if !self.is_valid_node() || !other.is_valid_node() {
            return false;
        }
        self.value().eq(other.value())
    }
}

impl Eq for Key {}

impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if !self.is_valid_node() {
            return 0.hash(state);
        }

        self.value().hash(state)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Map {
    pub(crate) lookup: HashMap<Key, Node>,
    pub(crate) all: Vec<(Key, Node)>,
}

impl Map {
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

impl FromIterator<(Key, Node)> for Map {
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

#[derive(Debug)]
pub(crate) struct AnnotationsInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) members: Shared<Map>,
}

#[derive(Debug, Clone)]
pub struct Annotations {
    inner: Arc<AnnotationsInner>,
}

impl Annotations {
    pub fn get(&self, key: &Key) -> Option<Node> {
        let members = self.inner.members.read();
        members.lookup.get(key).cloned()
    }

    pub fn value(&self) -> &Shared<Map> {
        &self.inner.members
    }

    pub fn members_keys(&self) -> Vec<StdString> {
        self.value()
            .read()
            .iter()
            .map(|(k, _)| k.to_string())
            .collect()
    }
}

impl Sealed for Annotations {}
impl DomNode for Annotations {
    fn node_syntax(&self) -> Option<&SyntaxElement> {
        None
    }

    fn syntax(&self) -> Option<&SyntaxElement> {
        None
    }

    fn errors(&self) -> &Shared<Vec<Error>> {
        &self.inner.errors
    }

    fn annotations(&self) -> Option<&Annotations> {
        None
    }

    fn validate_node(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

impl From<AnnotationsInner> for Annotations {
    fn from(inner: AnnotationsInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}
