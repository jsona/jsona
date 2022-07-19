use super::error::{Error, QueryError};
use super::index::Index;
use super::keys::{KeyOrIndex, Keys};
use crate::private::Sealed;
use crate::syntax::SyntaxElement;
use crate::util::quote::{quote, unquote, QuoteType};
use crate::util::shared::Shared;

use once_cell::unsync::OnceCell;
use rowan::{NodeOrToken, TextRange};
use serde_json::Number as JsonNumber;
use std::collections::HashMap;
use std::iter::{once, FromIterator};
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
            fn all_syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.all_syntax.as_ref()
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
    fn all_syntax(&self) -> Option<&SyntaxElement>;
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
    fn all_syntax(&self) -> Option<&SyntaxElement> {
        match self {
            $(
            Node::$elm(v) => v.all_syntax(),
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

    pub fn get(&self, idx: impl Index) -> Option<Node> {
        idx.index_into(self)
    }

    pub fn try_get(&self, idx: impl Index) -> Result<Node, QueryError> {
        idx.index_into(self).ok_or(QueryError::NotFound)
    }

    pub fn get_annotation<T: AsRef<str>>(&self, name: T) -> Option<Self> {
        self.annotations().and_then(|v| v.get(name))
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

    pub fn flat_iter(&self) -> impl DoubleEndedIterator<Item = (Keys, Node)> {
        let mut all = Vec::new();

        match self {
            Node::Object(t) => {
                let entries = t.inner.properties.read();
                for (key, entry) in &entries.all {
                    entry.collect_flat(
                        Keys::new(once(KeyOrIndex::PropertyKey(key.clone()))),
                        &mut all,
                    );
                }
            }
            Node::Array(arr) => {
                let items = arr.inner.items.read();
                for (idx, item) in items.iter().enumerate() {
                    item.collect_flat(Keys::from(idx), &mut all);
                }
            }
            _ => {}
        }

        if let Some(annotations) = self.annotations() {
            let entries = annotations.inner.entries.read();
            for (key, entry) in &entries.all {
                entry.collect_flat(
                    Keys::new(once(KeyOrIndex::AnnotationKey(key.clone()))),
                    &mut all,
                );
            }
        }

        all.into_iter()
    }

    pub fn annotation_iter(&self) -> impl DoubleEndedIterator<Item = (Keys, Node)> {
        let mut all = Vec::new();

        if let Some(annotations) = self.annotations() {
            let entries = annotations.inner.entries.read();
            for (key, entry) in &entries.all {
                all.push((
                    Keys::new(once(KeyOrIndex::AnnotationKey(key.clone()))),
                    entry.clone(),
                ))
            }
        }

        match self {
            Node::Object(t) => {
                let entries = t.inner.properties.read();
                for (key, entry) in &entries.all {
                    entry.collect_annotation(
                        Keys::new(once(KeyOrIndex::PropertyKey(key.clone()))),
                        &mut all,
                    );
                }
            }
            Node::Array(arr) => {
                let items = arr.inner.items.read();
                for (idx, item) in items.iter().enumerate() {
                    item.collect_annotation(Keys::from(idx), &mut all);
                }
            }
            _ => {}
        }

        all.into_iter()
    }

    pub fn matches_all(
        &self,
        keys: Keys,
        match_children: bool,
    ) -> Result<impl Iterator<Item = (Keys, Node)> + ExactSizeIterator, Error> {
        let all: Vec<(Keys, Node)> = self.flat_iter().collect();
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

    pub fn text_range(&self) -> Option<TextRange> {
        self.syntax().map(|v| v.text_range())
    }

    fn collect_flat(&self, parent: Keys, all: &mut Vec<(Keys, Node)>) {
        match self {
            Node::Object(t) => {
                all.push((parent.clone(), self.clone()));
                let entries = t.inner.properties.read();
                for (key, entry) in &entries.all {
                    entry.collect_flat(parent.join(KeyOrIndex::PropertyKey(key.clone())), all);
                }
            }
            Node::Array(arr) => {
                all.push((parent.clone(), self.clone()));
                let items = arr.inner.items.read();
                for (idx, item) in items.iter().enumerate() {
                    item.collect_flat(parent.join(KeyOrIndex::Index(idx)), all);
                }
            }
            _ => {
                all.push((parent.clone(), self.clone()));
            }
        }

        if let Some(annotations) = self.annotations() {
            let entries = annotations.inner.entries.read();
            for (key, entry) in &entries.all {
                entry.collect_flat(parent.join(KeyOrIndex::AnnotationKey(key.clone())), all);
            }
        }
    }

    fn collect_annotation(&self, parent: Keys, all: &mut Vec<(Keys, Node)>) {
        if let Some(annotations) = self.annotations() {
            let entries = annotations.inner.entries.read();
            for (key, entry) in &entries.all {
                all.push((
                    Keys::new(once(KeyOrIndex::AnnotationKey(key.clone()))),
                    entry.clone(),
                ))
            }
        }
        match self {
            Node::Object(t) => {
                all.push((parent.clone(), self.clone()));
                let entries = t.inner.properties.read();
                for (key, entry) in &entries.all {
                    entry
                        .collect_annotation(parent.join(KeyOrIndex::PropertyKey(key.clone())), all);
                }
            }
            Node::Array(arr) => {
                all.push((parent.clone(), self.clone()));
                let items = arr.inner.items.read();
                for (idx, item) in items.iter().enumerate() {
                    item.collect_annotation(parent.join(KeyOrIndex::Index(idx)), all);
                }
            }
            _ => {}
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

            let items = v.inner.entries.read();
            for (k, entry) in items.as_ref().all.iter() {
                if let Err(errs) = k.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
                entry.validate_all_impl(errors);
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

        pub fn $try_get_as_fn(&self, idx: impl Index) -> Result<Option<$t>, QueryError> {
            match idx.index_into(self) {
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

#[derive(Debug, Default)]
pub(crate) struct NullInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) all_syntax: Option<SyntaxElement>,
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
    pub(crate) all_syntax: Option<SyntaxElement>,
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
    pub(crate) all_syntax: Option<SyntaxElement>,
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
    pub(crate) all_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) repr: StrRepr,
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
                        StrRepr::Double => QuoteType::Double,
                        StrRepr::Single => QuoteType::Single,
                        StrRepr::Backtick => QuoteType::Backtick,
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
pub enum StrRepr {
    Single,
    Double,
    Backtick,
}

#[derive(Debug)]
pub(crate) struct ArrayInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) all_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) items: Shared<Vec<Node>>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Array { inner: ArrayInner }
}

impl Array {
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
    pub(crate) all_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) properties: Shared<Entries>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Object { inner: ObjectInner }
}

impl Object {
    pub fn get(&self, key: &Key) -> Option<Node> {
        let entries = self.inner.properties.read();
        entries.lookup.get(key).cloned()
    }

    pub fn value(&self) -> &Shared<Entries> {
        &self.inner.properties
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

impl<S> From<S> for Key
where
    S: Into<StdString>,
{
    fn from(s: S) -> Self {
        Key::new(s)
    }
}

impl Key {
    /// Return a new key with the given value.
    ///
    /// # Remarks
    ///
    /// This **does not** check or modify the input string.
    pub fn new(key: impl Into<StdString>) -> Self {
        KeyInner {
            errors: Default::default(),
            syntax: None,
            value: OnceCell::from(key.into()),
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

    pub fn text_range(&self) -> Option<TextRange> {
        self.syntax().map(|v| v.text_range())
    }

    pub fn text_ranges(&self) -> impl ExactSizeIterator<Item = TextRange> {
        self.text_range()
            .map(|v| vec![v].into_iter())
            .unwrap_or_else(|| vec![].into_iter())
    }
}

impl Sealed for Key {}
impl DomNode for Key {
    fn all_syntax(&self) -> Option<&SyntaxElement> {
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
        quote(self.value(), false).fmt(f)
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

#[derive(Debug)]
pub(crate) struct AnnotationsInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) entries: Shared<Entries>,
}

#[derive(Debug, Clone)]
pub struct Annotations {
    inner: Arc<AnnotationsInner>,
}

impl Annotations {
    pub fn get<T: AsRef<str>>(&self, key: T) -> Option<Node> {
        let entries = self.inner.entries.read();
        entries.lookup.get(&key.as_ref().into()).cloned()
    }

    pub fn value(&self) -> &Shared<Entries> {
        &self.inner.entries
    }
}

impl Sealed for Annotations {}
impl DomNode for Annotations {
    fn all_syntax(&self) -> Option<&SyntaxElement> {
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
