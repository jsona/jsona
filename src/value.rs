use indexmap::IndexMap;

pub enum Value {
    Null(Amap),
    Boolean(bool, Amap),
    Integer(i64, Amap),
    Float(f64, Amap),
    String(String, Amap),
    Array(Array, Amap),
    Object(Object, Amap),
}

pub type Array = Vec<Value>;

pub type Object = IndexMap<String, Value>;

pub type Amap = IndexMap<String, Vec<String>>;
