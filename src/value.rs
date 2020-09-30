use indexmap::IndexMap;

#[derive(Debug, PartialEq)]
pub enum Value {
    Null(Option<Amap>),
    Boolean(bool, Option<Amap>),
    Integer(i64, Option<Amap>),
    Float(f64, Option<Amap>),
    String(String, Option<Amap>),
    Array(Array, Option<Amap>),
    Object(Object, Option<Amap>),
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
            Value::Null(a) => a,
            Value::Boolean(_, a) => a,
            Value::Integer(_, a) => a,
            Value::Float(_, a) => a,
            Value::String(_, a) => a,
            Value::Array(_, a) => a,
            Value::Object(_, a) => a,
        }
    }
    pub fn get_annotations_mut(&mut self) -> &mut Option<Amap> {
        match self {
            Value::Null(a) => a,
            Value::Boolean(_, a) => a,
            Value::Integer(_, a) => a,
            Value::Float(_, a) => a,
            Value::String(_, a) => a,
            Value::Array(_, a) => a,
            Value::Object(_, a) => a,
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
