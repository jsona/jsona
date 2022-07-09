use super::keys::KeyOrIndex;
use super::node::{DomNode, Node};
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
            KeyOrIndex::Key(k) => {
                if let Node::Object(object) = v {
                    object.get(k.value())
                } else {
                    None
                }
            }
            KeyOrIndex::AnnoKey(k) => match v.annos() {
                Some(annos) => {
                    let entries = annos.entries().read();
                    entries.lookup.get(k).cloned()
                }
                None => None,
            },
        }
    }
}

impl Sealed for usize {}
impl Index for usize {
    fn index_into(&self, v: &Node) -> Option<Node> {
        if let Node::Array(arr) = v {
            let items = arr.items().read();
            items.get(*self).cloned()
        } else {
            None
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
