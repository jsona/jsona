use super::error::{Error, QueryError};
use super::from_syntax::comment_from_syntax;
use super::keys::{KeyOrIndex, Keys};
use crate::private::Sealed;
use crate::syntax::{SyntaxElement, SyntaxKind};
use crate::util::escape::unescape;
use crate::util::shared::Shared;

use either::Either;
use logos::Lexer;
use once_cell::unsync::OnceCell;
use rowan::{NodeOrToken, TextRange};
use std::collections::HashMap;
use std::fmt::Write;
use std::iter::{empty, once, FromIterator};
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

            fn annos(&self) -> &Option<$crate::dom::node::Annos> {
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
    fn annos(&self) -> &Option<Annos>;
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

    fn annos(&self) -> &Option<Annos> {
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
impl Node {
    pub fn path(&self, keys: &Keys) -> Option<Node> {
        let mut node = self.clone();
        for key in keys.iter() {
            node = node.get(key);
        }
        if node.is_invalid() {
            None
        } else {
            Some(node)
        }
    }

    pub fn get(&self, idx: &KeyOrIndex) -> Node {
        self.get_impl(idx).unwrap_or_else(|| {
            Node::from(
                InvalidInner {
                    errors: Shared::from(Vec::from([Error::Query(QueryError::NotFound)])),
                    syntax: None,
                    annos: None,
                }
                .wrap(),
            )
        })
    }

    pub fn try_get(&self, idx: &KeyOrIndex) -> Result<Node, Error> {
        self.get_impl(idx).ok_or(Error::Query(QueryError::NotFound))
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
                let entries = t.inner.entries.read();
                for (key, entry) in &entries.all {
                    entry.collect_flat(Keys::new(once(KeyOrIndex::new_key(key.clone()))), &mut all);
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

        if let Some(annos) = self.annos() {
            let entries = annos.inner.entries.read();
            for (key, entry) in &entries.all {
                entry.collect_flat(Keys::new(once(KeyOrIndex::new_key(key.clone()))), &mut all);
            }
        }

        all.into_iter()
    }

    pub fn find_all_matches(
        &self,
        keys: Keys,
        include_children: bool,
    ) -> Result<impl Iterator<Item = (Keys, Node)> + ExactSizeIterator, Error> {
        let mut all: Vec<(Keys, Node)> = self.flat_iter().collect();

        let mut err: Option<Error> = None;

        all.retain(|(k, _)| {
            if k.len() < keys.len() {
                return false;
            }

            let search_keys = keys.clone();
            let keys = k.clone();

            for (search_key, key) in search_keys.iter().zip(keys.iter()) {
                match search_key {
                    KeyOrIndex::Key(search_key) => {
                        let glob = match globset::Glob::new(search_key.value()) {
                            Ok(g) => g.compile_matcher(),
                            Err(glob_err) => {
                                err = Some(QueryError::from(glob_err).into());
                                return true;
                            }
                        };

                        match key {
                            KeyOrIndex::Key(key) => {
                                if !glob.is_match(key.value()) {
                                    return false;
                                }
                            }
                            KeyOrIndex::AnnoKey(_) => {
                                return false;
                            }
                            KeyOrIndex::Index(idx) => {
                                if !glob.is_match(&idx.to_string()) {
                                    return false;
                                }
                            }
                        }
                    }
                    KeyOrIndex::AnnoKey(search_key) => {
                        let glob = match globset::Glob::new(search_key.value()) {
                            Ok(g) => g.compile_matcher(),
                            Err(glob_err) => {
                                err = Some(QueryError::from(glob_err).into());
                                return true;
                            }
                        };

                        match key {
                            KeyOrIndex::Key(_) => {
                                return false;
                            }
                            KeyOrIndex::AnnoKey(key) => {
                                if !glob.is_match(key.value()) {
                                    return false;
                                }
                            }
                            KeyOrIndex::Index(_) => {
                                return false;
                            }
                        }
                    }
                    KeyOrIndex::Index(search_idx) => match key {
                        KeyOrIndex::Key(_) => {
                            return false;
                        }
                        KeyOrIndex::AnnoKey(_) => {
                            return false;
                        }
                        KeyOrIndex::Index(idx) => {
                            if idx != search_idx {
                                return false;
                            }
                        }
                    },
                }
            }

            true
        });

        if !include_children {
            all.retain(|(k, _)| k.len() == keys.len());
        }

        if let Some(err) = err {
            return Err(err);
        }

        Ok(all.into_iter())
    }

    pub fn text_ranges(&self) -> impl ExactSizeIterator<Item = TextRange> {
        let mut ranges = Vec::with_capacity(1);

        match self {
            Node::Object(v) => {
                let entries = v.entries().read();

                for (k, entry) in entries.iter() {
                    ranges.extend(k.text_ranges());
                    ranges.extend(entry.text_ranges());
                }

                if let Some(mut r) = v.syntax().map(|s| s.text_range()) {
                    for range in &ranges {
                        r = r.cover(*range);
                    }

                    ranges.insert(0, r);
                }
            }
            Node::Array(v) => {
                let items = v.items().read();
                for item in items.iter() {
                    ranges.extend(item.text_ranges());
                }

                if let Some(mut r) = v.syntax().map(|s| s.text_range()) {
                    for range in &ranges {
                        r = r.cover(*range);
                    }

                    ranges.insert(0, r);
                }
            }
            Node::Bool(v) => ranges.push(v.syntax().map(|s| s.text_range()).unwrap_or_default()),
            Node::Str(v) => ranges.push(v.syntax().map(|s| s.text_range()).unwrap_or_default()),
            Node::Integer(v) => ranges.push(v.syntax().map(|s| s.text_range()).unwrap_or_default()),
            Node::Float(v) => ranges.push(v.syntax().map(|s| s.text_range()).unwrap_or_default()),
            Node::Null(v) => ranges.push(v.syntax().map(|s| s.text_range()).unwrap_or_default()),
            Node::Invalid(v) => ranges.push(v.syntax().map(|s| s.text_range()).unwrap_or_default()),
        }

        if let Some(v) = self.annos() {
            let entries = v.entries().read();

            for (k, entry) in entries.iter() {
                ranges.extend(k.text_ranges());
                ranges.extend(entry.text_ranges());
            }

            if let Some(mut r) = v.syntax().map(|s| s.text_range()) {
                for range in &ranges {
                    r = r.cover(*range);
                }

                ranges.insert(0, r);
            }
        }

        ranges.into_iter()
    }

    /// All the comments in the tree, including header comments returned from [`Self::header_comments`].
    pub fn comments(&self) -> impl Iterator<Item = Comment> {
        if let Some(syntax) = self.syntax().cloned().and_then(|s| s.into_node()) {
            Either::Left(
                syntax
                    .descendants_with_tokens()
                    .filter(|t| {
                        t.kind() == SyntaxKind::COMMENT_BLOCK
                            || t.kind() == SyntaxKind::COMMENT_LINE
                    })
                    .map(comment_from_syntax),
            )
        } else {
            Either::Right(empty())
        }
    }

    /// Comments before the first item in the file.
    ///
    /// These are always counted from the root and the same
    /// values are returned from every node in the same tree.
    pub fn header_comments(&self) -> impl Iterator<Item = Comment> {
        let first_item = self
            .syntax()
            .and_then(|syntax| syntax.ancestors().last())
            .and_then(|root| root.descendants().nth(1));

        match first_item {
            Some(it) => Either::Left(self.comments().take_while(move |c| {
                c.syntax.as_ref().unwrap().text_range().end() <= it.text_range().start()
            })),
            None => Either::Right(self.comments()),
        }
    }

    fn get_impl(&self, idx: &KeyOrIndex) -> Option<Node> {
        match idx {
            KeyOrIndex::Index(v) => {
                if let Node::Array(arr) = self {
                    let items = arr.items().read();
                    items.get(*v).cloned()
                } else {
                    None
                }
            }
            KeyOrIndex::Key(k) => {
                if let Node::Object(obj) = self {
                    obj.get(k.clone())
                } else {
                    None
                }
            }
            KeyOrIndex::AnnoKey(k) => {
                if let Some(annos) = self.annos() {
                    annos.get(k.clone())
                } else {
                    None
                }
            }
        }
    }

    fn collect_flat(&self, parent: Keys, all: &mut Vec<(Keys, Node)>) {
        match self {
            Node::Object(t) => {
                all.push((parent.clone(), self.clone()));
                let entries = t.inner.entries.read();
                for (key, entry) in &entries.all {
                    entry.collect_flat(parent.join(KeyOrIndex::new_key(key.clone())), all);
                }
            }
            Node::Array(arr) => {
                all.push((parent.clone(), self.clone()));
                let items = arr.inner.items.read();
                for (idx, item) in items.iter().enumerate() {
                    item.collect_flat(parent.join(idx), all);
                }
            }
            _ => {
                all.push((parent.clone(), self.clone()));
            }
        }

        if let Some(annos) = self.annos() {
            all.push((parent.clone(), self.clone()));
            let entries = annos.inner.entries.read();
            for (key, entry) in &entries.all {
                entry.collect_flat(parent.join(KeyOrIndex::new_key(key.clone())), all);
            }
        }
    }

    fn validate_all_impl(&self, errors: &mut Vec<Error>) {
        match self {
            Node::Object(v) => {
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
            Node::Str(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
            Node::Integer(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
            Node::Float(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
            Node::Null(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
            Node::Invalid(v) => {
                if let Err(errs) = v.validate_node() {
                    errors.extend(errs.read().as_ref().iter().cloned())
                }
            }
        }
        if let Some(v) = self.annos() {
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

impl Node {
    /// Returns `true` if the node is [`Object`].
    ///
    /// [`Object`]: Node::Object
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(..))
    }

    /// Returns `true` if the node is [`Array`].
    ///
    /// [`Array`]: Node::Array
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(..))
    }

    /// Returns `true` if the node is [`Bool`].
    ///
    /// [`Bool`]: Node::Bool
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(..))
    }

    /// Returns `true` if the node is [`Str`].
    ///
    /// [`Str`]: Node::Str
    pub fn is_str(&self) -> bool {
        matches!(self, Self::Str(..))
    }

    /// Returns `true` if the node is [`Integer`].
    ///
    /// [`Integer`]: Node::Integer
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(..))
    }

    /// Returns `true` if the node is [`Float`].
    ///
    /// [`Float`]: Node::Float
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(..))
    }

    /// Returns `true` if the node is [`Null`].
    ///
    /// [`Null`]: Node::Null
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null(..))
    }

    /// Returns `true` if the node is [`Invalid`].
    ///
    /// [`Invalid`]: Node::Invalid
    pub fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid(..))
    }

    pub fn as_table(&self) -> Option<&Object> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&Array> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<&Bool> {
        if let Self::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&Str> {
        if let Self::Str(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_integer(&self) -> Option<&Integer> {
        if let Self::Integer(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<&Float> {
        if let Self::Float(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_null(&self) -> Option<&Null> {
        if let Self::Null(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_invalid(&self) -> Option<&Invalid> {
        if let Self::Invalid(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn try_into_object(self) -> Result<Object, Self> {
        if let Self::Object(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_array(self) -> Result<Array, Self> {
        if let Self::Array(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_bool(self) -> Result<Bool, Self> {
        if let Self::Bool(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_str(self) -> Result<Str, Self> {
        if let Self::Str(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_integer(self) -> Result<Integer, Self> {
        if let Self::Integer(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_float(self) -> Result<Float, Self> {
        if let Self::Float(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_null(self) -> Result<Null, Self> {
        if let Self::Null(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_invalid(self) -> Result<Invalid, Self> {
        if let Self::Invalid(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }
}

impl From<Null> for Node {
    fn from(v: Null) -> Self {
        Self::Null(v)
    }
}

impl From<Float> for Node {
    fn from(v: Float) -> Self {
        Self::Float(v)
    }
}

impl From<Integer> for Node {
    fn from(v: Integer) -> Self {
        Self::Integer(v)
    }
}

impl From<Str> for Node {
    fn from(v: Str) -> Self {
        Self::Str(v)
    }
}

impl From<Bool> for Node {
    fn from(v: Bool) -> Self {
        Self::Bool(v)
    }
}

impl From<Array> for Node {
    fn from(v: Array) -> Self {
        Self::Array(v)
    }
}

impl From<Object> for Node {
    fn from(v: Object) -> Self {
        Self::Object(v)
    }
}

impl From<Invalid> for Node {
    fn from(v: Invalid) -> Self {
        Self::Invalid(v)
    }
}

#[derive(Debug)]
pub(crate) struct NullInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Option<Annos>,
    /// For tag anno like `@optional`
    pub(crate) is_omitted: bool,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Null { inner: NullInner }
}

impl Null {
    pub fn new(is_omitted: bool) -> Self {
        NullInner {
            errors: Default::default(),
            syntax: None,
            is_omitted,
            annos: None,
        }
        .wrap()
    }
    pub fn is_omitted(&self) -> bool {
        self.inner.is_omitted
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
pub(crate) struct BoolInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Option<Annos>,
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
    pub(crate) annos: Option<Annos>,
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
    pub(crate) annos: Option<Annos>,
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
    pub(crate) annos: Option<Annos>,
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
                        let string = string.strip_prefix('`').unwrap_or(string);
                        let string = match string.strip_prefix("\r\n") {
                            Some(s) => s,
                            None => string.strip_prefix('\n').unwrap_or(string),
                        };
                        let string = string.strip_suffix('`').unwrap_or(string);
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
    pub(crate) annos: Option<Annos>,
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
    pub(crate) annos: Option<Annos>,
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

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug)]
pub(crate) struct InvalidInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Option<Annos>,
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

#[derive(Debug)]
pub(crate) struct KeyInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Option<Annos>,
    pub(crate) is_valid: bool,
    pub(crate) value: OnceCell<String>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Key { inner: KeyInner }
}

impl<S> From<S> for Key
where
    S: Into<String>,
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
    pub fn new(key: impl Into<String>) -> Self {
        KeyInner {
            errors: Default::default(),
            syntax: None,
            annos: None,
            is_valid: true,
            value: OnceCell::from(key.into()),
        }
        .wrap()
    }

    /// An unescaped value of the key.
    pub fn value(&self) -> &str {
        self.inner.value.get_or_init(|| {
            self.inner
                .syntax
                .as_ref()
                .and_then(NodeOrToken::as_token)
                .map(|s| {
                    if s.text().starts_with('\'') {
                        let string = s.text();
                        let string = string.strip_prefix('\'').unwrap_or(string);
                        let string = string.strip_suffix('\'').unwrap_or(string);
                        string.to_string()
                    } else if s.text().starts_with('"') {
                        let string = s.text();
                        let string = string.strip_prefix('"').unwrap_or(string);
                        let string = string.strip_suffix('"').unwrap_or(string);
                        match unescape(string) {
                            Ok(s) => s,
                            Err(_) => {
                                self.inner.errors.update(|errors| {
                                    errors.push(Error::InvalidEscapeSequence {
                                        string: s.clone().into(),
                                    })
                                });
                                String::new()
                            }
                        }
                    } else {
                        s.text().to_string()
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
            .map(|v| Either::Left(once(v)))
            .unwrap_or_else(|| Either::Right(empty()))
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if !self.inner.is_valid {
            return Err(self.errors());
        }

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
        if let Some(s) = &self.inner.syntax {
            return s.fmt(f);
        }

        if !matches!(
            Lexer::<SyntaxKind>::new(self.value()).next(),
            Some(SyntaxKind::IDENT) | None
        ) {
            f.write_char('\"')?;
            self.value().fmt(f)?;
            f.write_char('\"')?;
            return Ok(());
        }

        self.value().fmt(f)
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        if !self.inner.is_valid || !other.inner.is_valid {
            return false;
        }

        self.value().eq(other.value())
    }
}

impl Eq for Key {}

impl std::hash::Hash for Key {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if !self.inner.is_valid {
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
pub(crate) struct AnnosInner {
    pub(crate) errors: Shared<Vec<Error>>,
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) annos: Option<Annos>,
    pub(crate) entries: Shared<Entries>,
}

wrap_node! {
    #[derive(Debug, Clone)]
    pub struct Annos { inner: AnnosInner }
}

impl Annos {
    pub fn get(&self, key: impl Into<Key>) -> Option<Node> {
        let key = key.into();
        let entries = self.inner.entries.read();
        entries.lookup.get(&key).cloned()
    }

    pub fn entries(&self) -> &Shared<Entries> {
        &self.inner.entries
    }

    fn validate_impl(&self) -> Result<(), &Shared<Vec<Error>>> {
        if self.errors().read().as_ref().is_empty() {
            Ok(())
        } else {
            Err(self.errors())
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Comment {
    pub(crate) syntax: Option<SyntaxElement>,
    pub(crate) value: OnceCell<CommentValue>,
}

impl Comment {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            syntax: None,
            value: OnceCell::from(CommentValue::Comment(value.into())),
        }
    }

    pub fn new_directive(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            syntax: None,
            value: OnceCell::from(CommentValue::Directive {
                name: name.into(),
                value: value.into(),
            }),
        }
    }

    pub fn is_directive(&self) -> bool {
        self.value_internal().is_directive()
    }

    pub fn directive(&self) -> Option<&str> {
        if let CommentValue::Directive { name, .. } = self.value_internal() {
            Some(name)
        } else {
            None
        }
    }

    pub fn value(&self) -> &str {
        match self.value_internal() {
            CommentValue::Comment(s) => s,
            CommentValue::Directive { value, .. } => value,
        }
    }

    fn value_internal(&self) -> &CommentValue {
        self.value
            .get_or_init(|| match self.syntax.as_ref().and_then(|s| s.as_token()) {
                Some(t) => {
                    let text = t.text();

                    if let Some(directive_content) = text.strip_prefix("#:") {
                        let mut directive_content = directive_content.split_whitespace();
                        let directive_name = directive_content.next().unwrap_or("");
                        let directive_value = directive_content.next().unwrap_or("");
                        return CommentValue::Directive {
                            name: directive_name.into(),
                            value: directive_value.into(),
                        };
                    }

                    if let Some(comment_content) = text.strip_prefix('#') {
                        return CommentValue::Comment(comment_content.into());
                    }

                    Default::default()
                }
                None => Default::default(),
            })
    }
}

impl core::fmt::Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = &self.syntax {
            s.fmt(f)
        } else {
            match self.value_internal() {
                CommentValue::Comment(c) => {
                    f.write_str("#")?;
                    c.fmt(f)
                }
                CommentValue::Directive { name, value } => {
                    f.write_str("#:")?;
                    name.fmt(f)?;
                    f.write_str(" ")?;
                    value.fmt(f)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommentValue {
    Comment(String),
    Directive { name: String, value: String },
}

impl CommentValue {
    /// Returns `true` if the comment value is [`Directive`].
    ///
    /// [`Directive`]: CommentValue::Directive
    fn is_directive(&self) -> bool {
        matches!(self, Self::Directive { .. })
    }
}

impl Default for CommentValue {
    fn default() -> Self {
        Self::Comment(String::new())
    }
}
