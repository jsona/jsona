use indexmap::IndexMap;

#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

use crate::lexer::Position;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Doc {
    pub value: Value,
    pub annotation: Option<Amap>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum Value {
    Null {
        annotations: Option<Amap>,
        position: Position,
    },
    Boolean {
        value: bool,
        annotations: Option<Amap>,
        position: Position,
    },
    Integer {
        value: i64,
        annotations: Option<Amap>,
        position: Position,
    },
    Float {
        value: f64,
        annotations: Option<Amap>,
        position: Position,
    },
    String {
        value: String,
        annotations: Option<Amap>,
        position: Position,
    },
    Array {
        value: Array,
        annotations: Option<Amap>,
        position: Position,
    },
    Object {
        value: Object,
        annotations: Option<Amap>,
        position: Position,
    },
}

pub type Array = Vec<Value>;

pub type Object = IndexMap<String, (Position, Value)>;

pub type Amap = IndexMap<String, (Position, IndexMap<String, (Position, String)>)>;

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
            Value::Null { annotations, .. } => annotations,
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
            Value::Null { annotations, .. } => annotations,
            Value::Boolean { annotations, .. } => annotations,
            Value::Integer { annotations, .. } => annotations,
            Value::Float { annotations, .. } => annotations,
            Value::String { annotations, .. } => annotations,
            Value::Array { annotations, .. } => annotations,
            Value::Object { annotations, .. } => annotations,
        }
    }
}

