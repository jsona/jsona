use super::{
    error::Error,
    node::{
        Array, ArrayInner, ArrayKind, Bool, BoolInner, DomNode, 
        Null, NullInner, Float, FloatInner, Integer, IntegerInner, IntegerRepr,
        Invalid, InvalidInner, Key, KeyInner, Node,
        Str, StrInner, StrRepr, Object, ObjectInner, ObjectKind,
    },
};

use crate::{
    private::Sealed,
    syntax::{SyntaxElement, SyntaxKind::*},
    util::{iter::ExactIterExt, shared::Shared},
};

pub fn from_syntax(syntax: SyntaxElement) -> Node {
    todo!()
}