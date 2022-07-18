//! The Value enum, a loosely typed way of representing any valid JSONA value.

mod from_node;
mod to_jsona;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Value {
    Null(Null),
    Bool(Bool),
    Number(Number),
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
    Number(NumberValue),
    Str(String),
    Array(Vec<PlainValue>),
    Object(IndexMap<String, PlainValue>),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Null {
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bool {
    pub value: bool,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Number {
    pub value: NumberValue,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Str {
    pub value: String,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Array {
    pub items: Vec<Value>,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Object {
    pub properties: IndexMap<String, Value>,
    pub annotations: IndexMap<String, PlainValue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum NumberValue {
    Negative(i64),
    Positive(u64),
    Float(f64),
}

impl Eq for NumberValue {}

impl NumberValue {
    /// Returns `true` if the integer value is [`Negative`].
    ///
    /// [`Negative`]: NumberValue::Negative
    pub fn is_negative(&self) -> bool {
        matches!(self, Self::Negative(..))
    }

    /// Returns `true` if the integer value is [`Positive`].
    ///
    /// [`Positive`]: NumberValue::Positive
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Positive(..))
    }

    /// Returns `true` if the float value is [`Float`].
    ///
    /// [`Float`]: NumberValue::Float
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(..))
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

    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

impl core::fmt::Display for NumberValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberValue::Negative(v) => v.fmt(f),
            NumberValue::Positive(v) => v.fmt(f),
            NumberValue::Float(v) => v.fmt(f),
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

value_from!(Null, Number, Str, Bool, Array, Object,);

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
    define_value_fns!(Number, Number, is_number, as_nubmer);
    define_value_fns!(Str, Str, is_str, as_str);
    define_value_fns!(Object, Object, is_object, as_object);
    define_value_fns!(Array, Array, is_array, as_array);

    pub fn new_array(arr: impl IntoIterator<Item = Self>) -> Self {
        Array {
            annotations: Default::default(),
            items: arr.into_iter().collect(),
        }
        .into()
    }
    pub fn new_object(obj: impl IntoIterator<Item = (String, Self)>) -> Self {
        Object {
            annotations: Default::default(),
            properties: obj.into_iter().collect(),
        }
        .into()
    }

    pub fn annotations(&self) -> &IndexMap<String, PlainValue> {
        match self {
            Value::Null(Null { annotations, .. }) => annotations,
            Value::Bool(Bool { annotations, .. }) => annotations,
            Value::Number(Number { annotations, .. }) => annotations,
            Value::Str(Str { annotations, .. }) => annotations,
            Value::Array(Array { annotations, .. }) => annotations,
            Value::Object(Object { annotations, .. }) => annotations,
        }
    }
    pub fn annotations_mut(&mut self) -> &mut IndexMap<String, PlainValue> {
        match self {
            Value::Null(Null { annotations, .. }) => annotations,
            Value::Bool(Bool { annotations, .. }) => annotations,
            Value::Number(Number { annotations, .. }) => annotations,
            Value::Str(Str { annotations, .. }) => annotations,
            Value::Array(Array { annotations, .. }) => annotations,
            Value::Object(Object { annotations, .. }) => annotations,
        }
    }
}

impl From<PlainValue> for Value {
    fn from(annotation: PlainValue) -> Self {
        let annotations = Default::default();
        match annotation {
            PlainValue::Null => Null { annotations }.into(),
            PlainValue::Bool(value) => Bool { value, annotations }.into(),
            PlainValue::Number(value) => Number { value, annotations }.into(),
            PlainValue::Str(value) => Str { value, annotations }.into(),
            PlainValue::Array(value) => Array {
                items: value.into_iter().map(|v| v.into()).collect(),
                annotations,
            }
            .into(),
            PlainValue::Object(value) => Object {
                properties: value.into_iter().map(|(k, v)| (k, v.into())).collect(),
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
    define_annotation_value_fns!(Number, NumberValue, is_number, as_number);
    define_annotation_value_fns!(Str, String, is_str, as_str);
    define_annotation_value_fns!(Object, IndexMap<String, PlainValue>, is_object, as_object);
    define_annotation_value_fns!(Array, Vec<PlainValue>, is_array, as_array);
    pub fn new_array(arr: impl IntoIterator<Item = Self>) -> Self {
        PlainValue::Array(arr.into_iter().collect())
    }
    pub fn new_object(obj: impl IntoIterator<Item = (String, Self)>) -> Self {
        PlainValue::Object(obj.into_iter().collect())
    }
}

impl From<Value> for PlainValue {
    fn from(annotation: Value) -> Self {
        match annotation {
            Value::Null(_) => PlainValue::Null,
            Value::Bool(v) => PlainValue::Bool(v.value),
            Value::Number(v) => PlainValue::Number(v.value),
            Value::Str(v) => PlainValue::Str(v.value),
            Value::Array(v) => PlainValue::Array(v.items.into_iter().map(|v| v.into()).collect()),
            Value::Object(v) => {
                PlainValue::Object(v.properties.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}
