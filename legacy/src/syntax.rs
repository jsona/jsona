use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::string;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Jsona {
    Null(Null),
    Boolean(Boolean),
    Integer(Integer),
    Float(Float),
    String(String),
    Array(Array),
    Object(Object),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Null {
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Boolean {
    pub value: bool,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Integer {
    pub value: i64,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Float {
    pub value: f64,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct String {
    pub value: string::String,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Array {
    pub elements: Vec<Jsona>,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Object {
    pub properties: Vec<Property>,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Property {
    pub key: string::String,
    pub position: Position,
    pub value: Jsona,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Annotation {
    pub name: string::String,
    pub position: Position,
    pub value: Value,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Deserialize, Serialize)]
pub struct Position {
    pub index: usize,
    pub line: usize,
    pub col: usize,
}
impl Default for Position {
    fn default() -> Self {
        Self {
            index: 0,
            line: 1,
            col: 1,
        }
    }
}

impl Position {
    pub fn new(index: usize, line: usize, col: usize) -> Self {
        Position { index, line, col }
    }
}

macro_rules! define_is (
    ($name:ident, $yt:ident) => (
pub fn $name(&self) -> bool {
    match self {
        Jsona::$yt($yt { .. }) => true,
        _ => false
    }
}
    );
);

macro_rules! define_as_ref (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match self {
        Jsona::$yt(ref v) => Some(v),
        _ => None
    }
}
    );
);

impl Jsona {
    define_is!(is_null, Null);
    define_is!(is_boolean, Boolean);
    define_is!(is_integer, Integer);
    define_is!(is_float, Float);
    define_is!(is_string, String);
    define_is!(is_array, Array);
    define_is!(is_object, Object);

    define_as_ref!(as_boolean, &Boolean, Boolean);
    define_as_ref!(as_integer, &Integer, Integer);
    define_as_ref!(as_float, &Float, Float);
    define_as_ref!(as_string, &String, String);
    define_as_ref!(as_array, &Array, Array);
    define_as_ref!(as_object, &Object, Object);

    pub fn key(&self, key: &str) -> Option<&Self> {
        match self {
            Jsona::Object(Object {
                properties: value, ..
            }) => value.iter().find(|p| p.key == key).map(|v| &v.value),
            Jsona::Array(Array {
                elements: value, ..
            }) => {
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

    pub fn get_position(&self) -> &Position {
        match self {
            Jsona::Null(Null { position, .. }) => position,
            Jsona::Boolean(Boolean { position, .. }) => position,
            Jsona::Integer(Integer { position, .. }) => position,
            Jsona::Float(Float { position, .. }) => position,
            Jsona::String(String { position, .. }) => position,
            Jsona::Array(Array { position, .. }) => position,
            Jsona::Object(Object { position, .. }) => position,
        }
    }
    pub fn get_annotations(&self) -> &Vec<Annotation> {
        match self {
            Jsona::Null(Null { annotations, .. }) => annotations,
            Jsona::Boolean(Boolean { annotations, .. }) => annotations,
            Jsona::Integer(Integer { annotations, .. }) => annotations,
            Jsona::Float(Float { annotations, .. }) => annotations,
            Jsona::String(String { annotations, .. }) => annotations,
            Jsona::Array(Array { annotations, .. }) => annotations,
            Jsona::Object(Object { annotations, .. }) => annotations,
        }
    }
    pub fn get_annotations_mut(&mut self) -> &mut Vec<Annotation> {
        match self {
            Jsona::Null(Null { annotations, .. }) => annotations,
            Jsona::Boolean(Boolean { annotations, .. }) => annotations,
            Jsona::Integer(Integer { annotations, .. }) => annotations,
            Jsona::Float(Float { annotations, .. }) => annotations,
            Jsona::String(String { annotations, .. }) => annotations,
            Jsona::Array(Array { annotations, .. }) => annotations,
            Jsona::Object(Object { annotations, .. }) => annotations,
        }
    }
}

impl From<&Jsona> for Value {
    fn from(node: &Jsona) -> Self {
        match node {
            Jsona::Null(..) => Value::Null,
            Jsona::Boolean(Boolean { value, .. }) => value.to_owned().into(),
            Jsona::Integer(Integer { value, .. }) => value.to_owned().into(),
            Jsona::Float(Float { value, .. }) => value.to_owned().into(),
            Jsona::String(String { value, .. }) => value.to_owned().into(),
            Jsona::Array(Array {
                elements: value, ..
            }) => Value::Array(value.into_iter().map(|v| v.into()).collect()),
            Jsona::Object(Object {
                properties: value, ..
            }) => Value::Object(
                value
                    .into_iter()
                    .map(|v| (v.key.to_owned(), Value::from(&v.value)))
                    .collect::<Map<string::String, Value>>(),
            ),
        }
    }
}

impl From<Jsona> for Value {
    fn from(node: Jsona) -> Self {
        match node {
            Jsona::Null(..) => Value::Null,
            Jsona::Boolean(Boolean { value, .. }) => value.into(),
            Jsona::Integer(Integer { value, .. }) => value.into(),
            Jsona::Float(Float { value, .. }) => value.into(),
            Jsona::String(String { value, .. }) => value.into(),
            Jsona::Array(Array {
                elements: value, ..
            }) => Value::Array(value.into_iter().map(|v| v.into()).collect()),
            Jsona::Object(Object {
                properties: value, ..
            }) => Value::Object(
                value
                    .into_iter()
                    .map(|v| (v.key, v.value.into()))
                    .collect::<Map<string::String, Value>>(),
            ),
        }
    }
}
