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
    pub fn set_annotiaons(&mut self, annotiaons: Option<Amap>) {
        match annotiaons {
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
        }
    }
}

pub type Array = Vec<Value>;

pub type Object = IndexMap<String, Value>;

pub type Amap = IndexMap<String, Vec<String>>;
