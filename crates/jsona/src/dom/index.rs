use super::{node::Node, DomNode, Key, KeyOrIndex};
use crate::private::Sealed;

pub trait Index: Sealed + core::fmt::Display {
    #[doc(hidden)]
    fn index_into(&self, v: &Node) -> Option<Node>;
}

impl Sealed for KeyOrIndex {}
impl Index for KeyOrIndex {
    fn index_into(&self, v: &Node) -> Option<Node> {
        match self {
            KeyOrIndex::Index(idx) => idx.index_into(v),
            KeyOrIndex::PropertyKey(k) => v.as_object().and_then(|v| v.get(k)),
            KeyOrIndex::AnnotationKey(k) => v.annotations().and_then(|v| v.get(k)),
            _ => None,
        }
    }
}

impl Sealed for usize {}
impl Index for usize {
    fn index_into(&self, v: &Node) -> Option<Node> {
        if let Node::Array(arr) = v {
            let items = arr.value().read();
            items.get(*self).cloned()
        } else {
            None
        }
    }
}

impl Sealed for str {}
impl Index for str {
    fn index_into(&self, v: &Node) -> Option<Node> {
        self.to_string().index_into(v)
    }
}

impl Sealed for String {}
impl Index for String {
    fn index_into(&self, v: &Node) -> Option<Node> {
        let key = Key::new(self);
        if key.is_annotation() {
            KeyOrIndex::AnnotationKey(key).index_into(v)
        } else {
            KeyOrIndex::PropertyKey(key).index_into(v)
        }
    }
}

impl<'a, T> Sealed for &'a T where T: ?Sized + Sealed {}
impl<'a, T> Index for &'a T
where
    T: ?Sized + Index,
{
    fn index_into(&self, v: &Node) -> Option<Node> {
        (**self).index_into(v)
    }
}
