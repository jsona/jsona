//! The Value enum, a loosely typed way of representing any valid JSONA value.

mod to_jsona;

use crate::dom::{DomNode, Node};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Value {
    Null(Null),
    Bool(Bool),
    Integer(Integer),
    Float(Float),
    Str(Str),
    Array(Array),
    Object(Object),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum PlainValue {
    Null,
    Bool(bool),
    Integer(IntegerValue),
    Float(f64),
    Str(String),
    Array(Vec<PlainValue>),
    Object(IndexMap<String, PlainValue>),
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Null {
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bool {
    pub value: bool,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Integer {
    pub value: IntegerValue,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Float {
    pub value: f64,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Str {
    pub value: String,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Array {
    pub value: Vec<Value>,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Object {
    pub value: IndexMap<String, Value>,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
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

macro_rules! value_from {
    (
        $(
          $elm:ident,
        )*
    ) => {
    $(
    impl From<$elm> for Value {
        fn from(v: $elm) -> Self {
            Self::$elm(v)
        }
    }
    )*
    };
}

value_from!(Null, Float, Integer, Str, Bool, Array, Object,);

macro_rules! define_value_fns {
    ($elm:ident, $t:ty, $is_fn:ident, $as_fn:ident) => {
        pub fn $is_fn(&self) -> bool {
            match self {
                Value::$elm(_) => true,
                _ => false,
            }
        }
        pub fn $as_fn(&self) -> Option<&$t> {
            match self {
                Value::$elm(ref v) => Some(v),
                _ => None,
            }
        }
    };
}

impl Value {
    define_value_fns!(Null, Null, is_null, as_null);
    define_value_fns!(Bool, Bool, is_bool, as_bool);
    define_value_fns!(Integer, Integer, is_integer, as_integer);
    define_value_fns!(Float, Float, is_float, as_float);
    define_value_fns!(Str, Str, is_str, as_str);
    define_value_fns!(Object, Object, is_object, as_object);
    define_value_fns!(Array, Array, is_array, as_array);

    pub fn get_annotations(&self) -> &IndexMap<String, PlainValue> {
        match self {
            Value::Null(Null { annotations, .. }) => annotations,
            Value::Bool(Bool { annotations, .. }) => annotations,
            Value::Integer(Integer { annotations, .. }) => annotations,
            Value::Float(Float { annotations, .. }) => annotations,
            Value::Str(Str { annotations, .. }) => annotations,
            Value::Array(Array { annotations, .. }) => annotations,
            Value::Object(Object { annotations, .. }) => annotations,
        }
    }
    pub fn get_annotations_mut(&mut self) -> &mut IndexMap<String, PlainValue> {
        match self {
            Value::Null(Null { annotations, .. }) => annotations,
            Value::Bool(Bool { annotations, .. }) => annotations,
            Value::Integer(Integer { annotations, .. }) => annotations,
            Value::Float(Float { annotations, .. }) => annotations,
            Value::Str(Str { annotations, .. }) => annotations,
            Value::Array(Array { annotations, .. }) => annotations,
            Value::Object(Object { annotations, .. }) => annotations,
        }
    }
    pub fn from_node(node: &Node) -> Self {
        let mut annotations: IndexMap<String, PlainValue> = Default::default();
        if let Some(node_annotations) = node.annotations() {
            for (k, v) in node_annotations.entries().read().iter() {
                annotations.insert(k.value().to_string(), Value::from_node(v).into());
            }
        }
        match node {
            Node::Invalid(_) | Node::Null(_) => Null { annotations }.into(),
            Node::Bool(v) => Bool {
                value: v.value(),
                annotations,
            }
            .into(),
            Node::Integer(v) => Integer {
                value: v.value(),
                annotations,
            }
            .into(),
            Node::Float(v) => Float {
                value: v.value(),
                annotations,
            }
            .into(),
            Node::Str(v) => Str {
                value: v.value().to_string(),
                annotations,
            }
            .into(),
            Node::Array(v) => {
                let value = v.items().read().iter().map(|v| v.into()).collect();
                Array { value, annotations }.into()
            }
            Node::Object(v) => {
                let value = v
                    .entries()
                    .read()
                    .iter()
                    .map(|(k, v)| (k.value().to_string(), v.into()))
                    .collect();
                Object { value, annotations }.into()
            }
        }
    }
}

impl From<&Node> for Value {
    fn from(node: &Node) -> Self {
        Self::from_node(node)
    }
}

impl From<PlainValue> for Value {
    fn from(annotation: PlainValue) -> Self {
        let annotations = Default::default();
        match annotation {
            PlainValue::Null => Null { annotations }.into(),
            PlainValue::Bool(value) => Bool { value, annotations }.into(),
            PlainValue::Integer(value) => Integer { value, annotations }.into(),
            PlainValue::Float(value) => Float { value, annotations }.into(),
            PlainValue::Str(value) => Str { value, annotations }.into(),
            PlainValue::Array(value) => Array {
                value: value.into_iter().map(|v| v.into()).collect(),
                annotations,
            }
            .into(),
            PlainValue::Object(value) => Object {
                value: value.into_iter().map(|(k, v)| (k, v.into())).collect(),
                annotations,
            }
            .into(),
        }
    }
}

macro_rules! define_annotation_value_fns {
    ($yt:ident, $t:ty, $is_fn:ident,$as_fn:ident) => {
        pub fn $is_fn(&self) -> bool {
            match self {
                PlainValue::$yt(_) => true,
                _ => false,
            }
        }
        pub fn $as_fn(&self) -> Option<&$t> {
            match self {
                PlainValue::$yt(ref v) => Some(v),
                _ => None,
            }
        }
    };
}

impl PlainValue {
    pub fn is_null(&self) -> bool {
        matches!(self, PlainValue::Null)
    }
    pub fn as_null(&self) -> Option<()> {
        match self {
            PlainValue::Null => Some(()),
            _ => None,
        }
    }
    define_annotation_value_fns!(Bool, bool, is_bool, as_bool);
    define_annotation_value_fns!(Integer, IntegerValue, is_integer, as_integer);
    define_annotation_value_fns!(Float, f64, is_float, as_float);
    define_annotation_value_fns!(Str, String, is_str, as_str);
    define_annotation_value_fns!(Object, IndexMap<String, PlainValue>, is_object, as_object);
    define_annotation_value_fns!(Array, Vec<PlainValue>, is_array, as_array);
}

impl From<Value> for PlainValue {
    fn from(annotation: Value) -> Self {
        match annotation {
            Value::Null(_) => PlainValue::Null,
            Value::Bool(v) => PlainValue::Bool(v.value),
            Value::Integer(v) => PlainValue::Integer(v.value),
            Value::Float(v) => PlainValue::Float(v.value),
            Value::Str(v) => PlainValue::Str(v.value),
            Value::Array(v) => PlainValue::Array(v.value.into_iter().map(|v| v.into()).collect()),
            Value::Object(v) => {
                PlainValue::Object(v.value.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}
