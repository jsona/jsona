use indexmap::IndexMap;
use std::ops::Index;
use std::vec;

#[derive(Debug, PartialEq)]
pub enum Value {
    Null(Option<Amap>),
    Boolean(bool, Option<Amap>),
    Integer(i64, Option<Amap>),
    Float(f64, Option<Amap>),
    String(String, Option<Amap>),
    Array(Array, Option<Amap>),
    Object(Object, Option<Amap>),

    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue(Option<Amap>),
}

impl Value {
    pub fn is_scalar(&self) -> bool {
        match self {
            Value::Null(_)
            | Value::Boolean(_, _)
            | Value::Integer(_, _)
            | Value::Float(_, _)
            | Value::String(_, _) => true,
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
            Value::Null(annotations) => annotations,
            Value::Boolean(_, annotations) => annotations,
            Value::Integer(_, annotations) => annotations,
            Value::Float(_, annotations) => annotations,
            Value::String(_, annotations) => annotations,
            Value::Array(_, annotations) => annotations,
            Value::Object(_, annotations) => annotations,
            Value::BadValue(annotations) => annotations,
        }
    }
    pub fn get_annotations_mut(&mut self) -> &mut Option<Amap> {
        match self {
            Value::Null(annotations) => annotations,
            Value::Boolean(_, annotations) => annotations,
            Value::Integer(_, annotations) => annotations,
            Value::Float(_, annotations) => annotations,
            Value::String(_, annotations) => annotations,
            Value::Array(_, annotations) => annotations,
            Value::Object(_, annotations) => annotations,
            Value::BadValue(annotations) => annotations,
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
        Value::$yt(v, _) => Some(v),
        _ => None
    }
}
    );
);

macro_rules! define_as_ref (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(&self) -> Option<$t> {
    match *self {
        Value::$yt(ref v, _) => Some(v),
        _ => None
    }
}
    );
);

macro_rules! define_into (
    ($name:ident, $t:ty, $yt:ident) => (
pub fn $name(self) -> Option<$t> {
    match self {
        Value::$yt(v, _) => Some(v),
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
            Value::Null(_) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match *self {
            Value::Array(..) => true,
            _ => false,
        }
    }
    pub fn is_object(&self) -> bool {
        match *self {
            Value::Object(..) => true,
            _ => false,
        }
    }
}

static BAD_VALUE: Value = Value::BadValue(None);
impl<'a> Index<&'a str> for Value {
    type Output = Value;

    fn index(&self, idx: &'a str) -> &Value {
        match self.as_hash() {
            Some(h) => h.get(idx).unwrap_or(&BAD_VALUE),
            None => &BAD_VALUE,
        }
    }
}

impl Index<usize> for Value {
    type Output = Value;

    fn index(&self, idx: usize) -> &Value {
        if let Some(v) = self.as_vec() {
            v.get(idx).unwrap_or(&BAD_VALUE)
        } else if let Some(v) = self.as_hash() {
            let key = idx.to_string();
            v.get(key.as_str()).unwrap_or(&BAD_VALUE)
        } else {
            &BAD_VALUE
        }
    }
}

impl IntoIterator for Value {
    type Item = Value;
    type IntoIter = ValueIter;

    fn into_iter(self) -> Self::IntoIter {
        ValueIter {
            yaml: self.into_vec().unwrap_or_else(Vec::new).into_iter(),
        }
    }
}

pub struct ValueIter {
    yaml: vec::IntoIter<Value>,
}

impl Iterator for ValueIter {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        self.yaml.next()
    }
}
