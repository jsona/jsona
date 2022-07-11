use crate::dom::{DomNode, Node};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Null {
    pub annotations: IndexMap<String, Value>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bool {
    pub value: bool,
    pub annotations: IndexMap<String, Value>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Integer {
    pub value: IntegerValue,
    pub annotations: IndexMap<String, Value>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Float {
    pub value: f64,
    pub annotations: IndexMap<String, Value>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Str {
    pub value: String,
    pub annotations: IndexMap<String, Value>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Array {
    pub value: Vec<Value>,
    pub annotations: IndexMap<String, Value>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Object {
    pub value: IndexMap<String, Value>,
    pub annotations: IndexMap<String, Value>,
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

impl From<Null> for Value {
    fn from(v: Null) -> Self {
        Self::Null(v)
    }
}

impl From<Float> for Value {
    fn from(v: Float) -> Self {
        Self::Float(v)
    }
}

impl From<Integer> for Value {
    fn from(v: Integer) -> Self {
        Self::Integer(v)
    }
}

impl From<Str> for Value {
    fn from(v: Str) -> Self {
        Self::Str(v)
    }
}

impl From<Bool> for Value {
    fn from(v: Bool) -> Self {
        Self::Bool(v)
    }
}

impl From<Array> for Value {
    fn from(v: Array) -> Self {
        Self::Array(v)
    }
}

impl From<Object> for Value {
    fn from(v: Object) -> Self {
        Self::Object(v)
    }
}

macro_rules! define_is (
    ($name:ident, $yt:ident) => (
pub fn $name(&self) -> bool {
    match self {
        Value::$yt($yt { .. }) => true,
        _ => false
    }
}
    );
);

macro_rules! define_as_ref (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match self {
        Value::$yt(ref v) => Some(v),
        _ => None
    }
}
    );
);

impl Value {
    define_is!(is_null, Null);
    define_is!(is_boolean, Bool);
    define_is!(is_integer, Integer);
    define_is!(is_float, Float);
    define_is!(is_str, Str);
    define_is!(is_array, Array);
    define_is!(is_object, Object);

    define_as_ref!(as_boolean, &Bool, Bool);
    define_as_ref!(as_integer, &Integer, Integer);
    define_as_ref!(as_float, &Float, Float);
    define_as_ref!(as_str, &Str, Str);
    define_as_ref!(as_array, &Array, Array);
    define_as_ref!(as_object, &Object, Object);

    pub fn key(&self, key: &str) -> Option<&Self> {
        match self {
            Value::Object(Object { value, .. }) => value.get(key),
            Value::Array(Array { value, .. }) => {
                if let Ok(idx) = key.parse::<usize>() {
                    value.get(idx)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub fn retrive(&self, path: &[&str]) -> Option<&Self> {
        path.iter()
            .fold(Some(self), |v, &b| v.and_then(|v| v.key(b)))
    }

    pub fn get_annotations(&self) -> &IndexMap<String, Value> {
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
    pub fn get_annotations_mut(&mut self) -> &mut IndexMap<String, Value> {
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
}

impl From<&Node> for Value {
    fn from(node: &Node) -> Self {
        let mut annotations: IndexMap<String, Value> = Default::default();
        if let Some(node_annotations) = node.annotations() {
            for (k, v) in node_annotations.entries().read().iter() {
                annotations.insert(k.value().to_string(), v.into());
            }
        }
        match node {
            Node::Null(_) => Null { annotations }.into(),
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
            Node::Invalid(_) => Null { annotations }.into(),
        }
    }
}
