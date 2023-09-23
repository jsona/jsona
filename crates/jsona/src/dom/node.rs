use super::error::DomError;
use super::keys::{KeyOrIndex, Keys};
use super::query_keys::QueryKeys;
use super::visitor::{VisitControl, Visitor};
use crate::error::Error as JsonaError;
use crate::parser;
use crate::private::Sealed;
use crate::syntax::SyntaxElement;
use crate::util::mapper;
use crate::util::shared::Shared;
use crate::util::{check_quote, quote, unquote};

use indexmap::IndexMap;
use once_cell::unsync::OnceCell;
use rowan::{NodeOrToken, TextRange};
use serde_json::Number as JsonNumber;
use std::rc::Rc;
use std::str::FromStr;
use std::string::String as StdString;

pub trait DomNode: Sized + Sealed {
    fn node_syntax(&self) -> Option<&SyntaxElement>;
    fn syntax(&self) -> Option<&SyntaxElement>;
    fn errors(&self) -> &Shared<Vec<DomError>>;
    fn annotations(&self) -> Option<&Annotations>;
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
        }
    }

    pub fn validate(&self) -> Result<(), impl Iterator<Item = DomError> + core::fmt::Debug> {
        let mut errors = Vec::new();
        self.validate_all_impl(&mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into_iter())
        }
    }

    pub fn is_valid(&self) -> bool {
        let mut valid = true;
        self.is_valid_impl(&mut valid);
        valid
    }

    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Node::Null(_) | Node::Bool(_) | Node::Number(_) | Node::String(_)
        )
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Self::Number(v) => v.is_integer(),
            _ => false,
        }
    }

    pub fn text_range(&self) -> Option<TextRange> {
        self.syntax().map(|v| v.text_range())
    }

    pub fn node_text_range(&self) -> Option<TextRange> {
        self.node_syntax().map(|v| v.text_range())
    }

    pub fn matches_all(
        &self,
        keys: QueryKeys,
        match_children: bool,
    ) -> Result<impl Iterator<Item = (Keys, Node)> + ExactSizeIterator, DomError> {
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
                if v.errors().read().is_empty() {
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

    pub fn mapper_range(&self, mapper: &mapper::Mapper) -> Option<mapper::Range> {
        self.syntax()
            .and_then(|syntax| mapper.range(syntax.text_range()))
    }

    fn is_valid_impl(&self, valid: &mut bool) {
        match self {
            Node::Object(v) => {
                if !v.errors().read().as_ref().is_empty() {
                    *valid = false;
                    return;
                }
                let items = v.inner.properties.read();
                for (k, entry) in items.as_ref().iter() {
                    if !k.is_valid() {
                        *valid = false;
                        return;
                    }
                    entry.is_valid_impl(valid);
                    if !*valid {
                        return;
                    }
                }
            }
            Node::Array(v) => {
                if !v.errors().read().as_ref().is_empty() {
                    *valid = false;
                    return;
                }
                let items = v.inner.items.read();
                for item in &**items.as_ref() {
                    item.is_valid_impl(valid);
                    if !*valid {
                        return;
                    }
                }
            }
            Node::Bool(v) => {
                if !v.errors().read().as_ref().is_empty() {
                    *valid = false;
                    return;
                }
            }
            Node::String(v) => {
                if !v.errors().read().as_ref().is_empty() {
                    *valid = false;
                    return;
                }
            }
            Node::Number(v) => {
                if !v.errors().read().as_ref().is_empty() {
                    *valid = false;
                    return;
                }
            }
            Node::Null(v) => {
                if !v.errors().read().as_ref().is_empty() {
                    *valid = false;
                    return;
                }
            }
        }
        if let Some(v) = self.annotations() {
            if !v.errors().read().as_ref().is_empty() {
                *valid = false;
                return;
            }
            let items = v.inner.map.read();
            for (k, node) in items.as_ref().iter() {
                if !k.errors().read().as_ref().is_empty() {
                    *valid = false;
                    return;
                }
                node.is_valid_impl(valid);
                if !*valid {
                    return;
                }
            }
        }
    }

    fn validate_all_impl(&self, errors: &mut Vec<DomError>) {
        match self {
            Node::Object(v) => {
                errors.extend(v.errors().read().as_ref().iter().cloned());

                let items = v.inner.properties.read();
                for (k, entry) in items.as_ref().iter() {
                    errors.extend(k.errors().read().as_ref().iter().cloned());
                    entry.validate_all_impl(errors);
                }
            }
            Node::Array(v) => {
                errors.extend(v.errors().read().as_ref().iter().cloned());
                let items = v.inner.items.read();
                for item in &**items.as_ref() {
                    item.validate_all_impl(errors);
                }
            }
            Node::Bool(v) => {
                errors.extend(v.errors().read().as_ref().iter().cloned());
            }
            Node::String(v) => {
                errors.extend(v.errors().read().as_ref().iter().cloned());
            }
            Node::Number(v) => {
                errors.extend(v.errors().read().as_ref().iter().cloned());
            }
            Node::Null(v) => {
                errors.extend(v.errors().read().as_ref().iter().cloned());
            }
        }
        if let Some(v) = self.annotations() {
            errors.extend(v.errors().read().as_ref().iter().cloned());
            let items = v.inner.map.read();
            for (k, node) in items.as_ref().iter() {
                errors.extend(k.errors().read().as_ref().iter().cloned());
                node.validate_all_impl(errors);
            }
        }
    }
}

impl Node {
    define_value_fns!(Null, Null, is_null, as_null, get_as_null);
    define_value_fns!(Bool, Bool, is_bool, as_bool, get_as_bool);
    define_value_fns!(Number, Number, is_number, as_number, get_as_number);
    define_value_fns!(String, String, is_string, as_string, get_as_string);
    define_value_fns!(Object, Object, is_object, as_object, get_as_object);
    define_value_fns!(Array, Array, is_array, as_array, get_as_array);
}

value_from!(Null, Number, String, Bool, Array, Object,);

impl FromStr for Node {
    type Err = JsonaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse = parser::parse(s);
        if !parse.errors.is_empty() {
            return Err(JsonaError::InvalidSyntax {
                errors: parse.errors,
            });
        }
        let root = parse.into_dom();
        if let Err(errors) = root.validate() {
            return Err(JsonaError::InvalidDom {
                errors: errors.collect(),
            });
        }
        Ok(root)
    }
}

#[derive(Debug, Default)]
pub(crate) struct NullInner {
    pub(crate) errors: Shared<Vec<DomError>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Null { inner: NullInner }
}

impl Null {
    pub fn new(annotations: Option<Annotations>) -> Self {
        NullInner {
            errors: Default::default(),
            syntax: None,
            node_syntax: None,
            annotations,
        }
        .into()
    }

    pub fn is_valid(&self) -> bool {
        Node::Null(self.clone()).is_valid()
    }
}

#[derive(Debug)]
pub(crate) struct BoolInner {
    pub(crate) errors: Shared<Vec<DomError>>,
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
    pub fn new(value: bool, annotations: Option<Annotations>) -> Self {
        BoolInner {
            errors: Default::default(),
            syntax: None,
            node_syntax: None,
            annotations,
            value: value.into(),
        }
        .into()
    }
    /// A boolean value.
    pub fn value(&self) -> bool {
        *self.inner.value.get_or_init(|| {
            self.syntax()
                .and_then(|s| s.as_token())
                .and_then(|s| s.text().parse().ok())
                .unwrap_or_default()
        })
    }
}

#[derive(Debug)]
pub(crate) struct NumberInner {
    pub(crate) errors: Shared<Vec<DomError>>,
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
    pub fn new(value: JsonNumber, annotations: Option<Annotations>) -> Self {
        NumberInner {
            errors: Default::default(),
            syntax: None,
            node_syntax: None,
            annotations,
            repr: Default::default(),
            value: value.into(),
        }
        .into()
    }

    /// An number value.
    pub fn value(&self) -> &JsonNumber {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .map(|s| {
                    let text = s.as_token().unwrap().text().replace('_', "");

                    match self.inner.repr {
                        NumberRepr::Float => {
                            match text.parse::<f64>().ok().and_then(JsonNumber::from_f64) {
                                Some(v) => v,
                                None => {
                                    self.inner.errors.update(|errors| {
                                        errors.push(DomError::InvalidNumber { syntax: s.clone() })
                                    });
                                    JsonNumber::from_f64(0.0).unwrap()
                                }
                            }
                        }
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
                    }
                })
                .unwrap_or_else(|| JsonNumber::from(0))
        })
    }
    pub fn is_integer(&self) -> bool {
        self.inner.repr != NumberRepr::Float
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NumberRepr {
    Dec,
    Bin,
    Oct,
    Hex,
    Float,
}

impl Default for NumberRepr {
    fn default() -> Self {
        Self::Float
    }
}

#[derive(Debug)]
pub(crate) struct StringInner {
    pub(crate) errors: Shared<Vec<DomError>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) node_syntax: Option<SyntaxElement>,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) value: OnceCell<StdString>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct String { inner: StringInner }
}

impl String {
    pub fn new(value: StdString, annotations: Option<Annotations>) -> Self {
        StringInner {
            errors: Default::default(),
            syntax: None,
            node_syntax: None,
            annotations,
            value: value.into(),
        }
        .into()
    }

    /// An unescaped value of the string.
    pub fn value(&self) -> &str {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .map(|s| match unquote(&s.to_string()) {
                    Ok(s) => s,
                    Err(_) => {
                        self.inner.errors.update(|errors| {
                            errors.push(DomError::InvalidString { syntax: s.clone() })
                        });
                        StdString::new()
                    }
                })
                .unwrap_or_default()
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum StringRepr {
    Double,
    Single,
    Backtick,
}

impl Default for StringRepr {
    fn default() -> Self {
        Self::Double
    }
}

#[derive(Debug)]
pub(crate) struct ArrayInner {
    pub(crate) errors: Shared<Vec<DomError>>,
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
    pub fn new(items: Vec<Node>, annotations: Option<Annotations>) -> Self {
        ArrayInner {
            errors: Default::default(),
            syntax: None,
            node_syntax: None,
            annotations,
            items: items.into(),
        }
        .into()
    }

    pub fn get(&self, idx: usize) -> Option<Node> {
        let items = self.inner.items.read();
        items.get(idx).cloned()
    }

    pub fn value(&self) -> &Shared<Vec<Node>> {
        &self.inner.items
    }
}

#[derive(Debug)]
pub(crate) struct ObjectInner {
    pub(crate) errors: Shared<Vec<DomError>>,
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
    pub fn new(properties: Map, annotations: Option<Annotations>) -> Self {
        ObjectInner {
            errors: Default::default(),
            syntax: None,
            node_syntax: None,
            annotations,
            properties: properties.into(),
        }
        .into()
    }

    pub fn get(&self, key: &Key) -> Option<Node> {
        let props = self.inner.properties.read();
        props.value.get(key).map(|(node, _)| node.clone())
    }

    pub fn value(&self) -> &Shared<Map> {
        &self.inner.properties
    }
}

#[derive(Debug)]
pub(crate) struct KeyInner {
    pub(crate) errors: Shared<Vec<DomError>>,
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
    inner: Rc<KeyInner>,
}

impl From<KeyInner> for Key {
    fn from(inner: KeyInner) -> Self {
        Self {
            inner: Rc::new(inner),
        }
    }
}

impl From<&str> for Key {
    fn from(v: &str) -> Self {
        if v.starts_with('@') {
            Self::annotation(v)
        } else {
            Self::property(v)
        }
    }
}

impl Key {
    pub fn syntax(&self) -> Option<&SyntaxElement> {
        self.inner.syntax.as_ref()
    }

    pub fn errors(&self) -> &Shared<Vec<DomError>> {
        &self.inner.errors
    }

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
                    match unquote(s.text()) {
                        Ok(s) => s,
                        Err(_) => {
                            self.inner.errors.update(|errors| {
                                errors.push(DomError::InvalidString {
                                    syntax: s.clone().into(),
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

    pub fn annotation_name(&self) -> Option<StdString> {
        if self.is_annotation() {
            Some(self.value()[1..].to_string())
        } else {
            None
        }
    }

    pub fn to_origin_string(&self) -> StdString {
        match self.syntax() {
            Some(v) => v.to_string(),
            None => self.value().to_string(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.errors().read().is_empty()
    }

    pub fn is_quote(&self) -> bool {
        check_quote(self.value()).is_some()
    }

    pub fn mapper_range(&self, mapper: &mapper::Mapper) -> Option<mapper::Range> {
        self.syntax()
            .and_then(|syntax| mapper.range(syntax.text_range()))
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
        if self.is_valid() && self.is_valid() {
            return self.value().eq(other.value());
        }
        false
    }
}

impl Eq for Key {}

impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_valid() {
            return self.value().hash(state);
        }
        0.hash(state);
    }
}

#[derive(Debug, Clone, Default)]
pub struct Map {
    pub(crate) value: IndexMap<Key, (Node, Option<SyntaxElement>)>,
}

impl Map {
    pub fn len(&self) -> usize {
        self.value.len()
    }

    pub fn syntax(&self, key: &Key) -> Option<SyntaxElement> {
        self.value.get(key).and_then(|(_, syntax)| syntax.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Node)> {
        self.value.iter().map(|(key, (node, _))| (key, node))
    }

    pub fn add(&mut self, key: Key, node: Node, syntax: Option<SyntaxElement>) {
        self.value.insert(key, (node, syntax));
    }
}

#[derive(Debug)]
pub(crate) struct AnnotationsInner {
    pub(crate) errors: Shared<Vec<DomError>>,
    pub(crate) map: Shared<Map>,
}

#[derive(Debug, Clone)]
pub struct Annotations {
    inner: Rc<AnnotationsInner>,
}

impl Annotations {
    pub fn new(map: Map) -> Self {
        AnnotationsInner {
            errors: Default::default(),
            map: map.into(),
        }
        .into()
    }

    pub fn errors(&self) -> &Shared<Vec<DomError>> {
        &self.inner.errors
    }

    pub fn get(&self, key: &Key) -> Option<Node> {
        let map = self.inner.map.read();
        map.value.get(key).map(|(node, _)| node.clone())
    }

    pub fn annotation_syntax(&self, key: &Key) -> Option<SyntaxElement> {
        let map = self.inner.map.read();
        map.value.get(key).and_then(|(_, syntax)| syntax.clone())
    }

    pub fn value(&self) -> &Shared<Map> {
        &self.inner.map
    }

    pub fn map_keys(&self) -> Vec<StdString> {
        self.value()
            .read()
            .iter()
            .map(|(k, _)| k.to_string())
            .collect()
    }
}

impl From<AnnotationsInner> for Annotations {
    fn from(inner: AnnotationsInner) -> Self {
        Self {
            inner: Rc::new(inner),
        }
    }
}
