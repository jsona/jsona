use indexmap::IndexMap;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Doc {
    pub value: Value,
    pub annotation: Option<Amap>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum Value {
    Null {
        annotations: Option<Amap>,
    },
    Boolean {
        value: bool,
        annotations: Option<Amap>,
    },
    Integer {
        value: i64,
        annotations: Option<Amap>,
    },
    Float {
        value: f64,
        annotations: Option<Amap>,
    },
    String {
        value: String,
        annotations: Option<Amap>,
    },
    Array {
        value: Array,
        annotations: Option<Amap>,
    },
    Object {
        value: Object,
        annotations: Option<Amap>,
    },
}

impl Value {
    pub fn is_scalar(&self) -> bool {
        match self {
            Value::Null { .. }
            | Value::Boolean { .. }
            | Value::Integer { .. }
            | Value::Float { .. }
            | Value::String { .. } => true,
            _ => false,
        }
    }
    pub fn set_annotations(&mut self, annotations: Option<Amap>) {
        match annotations {
            Some(v) => self.get_annotations_mut().replace(v),
            None => self.get_annotations_mut().take(),
        };
    }
    pub fn get_annotations(&self) -> &Option<Amap> {
        match self {
            Value::Null { annotations } => annotations,
            Value::Boolean { annotations, .. } => annotations,
            Value::Integer { annotations, .. } => annotations,
            Value::Float { annotations, .. } => annotations,
            Value::String { annotations, .. } => annotations,
            Value::Array { annotations, .. } => annotations,
            Value::Object { annotations, .. } => annotations,
        }
    }
    pub fn get_annotations_mut(&mut self) -> &mut Option<Amap> {
        match self {
            Value::Null { annotations } => annotations,
            Value::Boolean { annotations, .. } => annotations,
            Value::Integer { annotations, .. } => annotations,
            Value::Float { annotations, .. } => annotations,
            Value::String { annotations, .. } => annotations,
            Value::Array { annotations, .. } => annotations,
            Value::Object { annotations, .. } => annotations,
        }
    }
}

pub type Array = Vec<Value>;

pub type Object = IndexMap<String, Value>;

pub type Amap = IndexMap<String, IndexMap<String, String>>;

macro_rules! define_as (
    ($name:ident, $t:ident, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match *self {
        Value::$yt{ value, .. } => Some(value),
        _ => None
    }
}
    );
);

macro_rules! define_as_ref (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match *self {
        Value::$yt{ ref value, .. } => Some(value),
        _ => None
    }
}
    );
);

macro_rules! define_into (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(self) -> Option<$t> {
    match self {
        Value::$yt{ value, .. } => Some(value),
        _ => None
    }
}
    );
);

impl Value {
    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);
    define_as!(as_f64, f64, Float);

    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_hash, &Object, Object);
    define_as_ref!(as_vec, &Array, Array);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_f64, f64, Float);
    define_into!(into_i64, i64, Integer);
    define_into!(into_string, String, String);
    define_into!(into_hash, Object, Object);
    define_into!(into_vec, Array, Array);

    pub fn is_null(&self) -> bool {
        match *self {
            Value::Null { .. } => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match *self {
            Value::Array { .. } => true,
            _ => false,
        }
    }
    pub fn is_object(&self) -> bool {
        match *self {
            Value::Object { .. } => true,
            _ => false,
        }
    }
}
