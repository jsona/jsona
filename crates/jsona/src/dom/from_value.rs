use indexmap::IndexMap;

use crate::value::{self, PlainValue, Value};

use super::*;

impl Node {
    pub fn from_value(value: &Value) -> Self {
        let annotations = from_value_annotaions(value.annotations());

        match value {
            Value::Null(value::Null { .. }) => NullInner {
                annotations,
                ..Default::default()
            }
            .wrap()
            .into(),
            Value::Bool(value::Bool { value, .. }) => BoolInner {
                errors: Default::default(),
                syntax: None,
                value_syntax: None,
                annotations,
                value: (*value).into(),
            }
            .wrap()
            .into(),
            Value::Integer(value::Integer { value, .. }) => IntegerInner {
                errors: Default::default(),
                syntax: None,
                value_syntax: None,
                annotations,
                repr: IntegerRepr::Dec,
                value: (*value).into(),
            }
            .wrap()
            .into(),
            Value::Float(value::Float { value, .. }) => FloatInner {
                errors: Default::default(),
                syntax: None,
                value_syntax: None,
                annotations,
                value: (*value).into(),
            }
            .wrap()
            .into(),
            Value::Str(value::Str { value, .. }) => StrInner {
                errors: Default::default(),
                syntax: None,
                value_syntax: None,
                annotations,
                repr: StrRepr::Double,
                value: value.to_string().into(),
            }
            .wrap()
            .into(),
            Value::Array(value::Array { value, .. }) => {
                let items: Vec<Node> = value.iter().map(|v| v.into()).collect();
                ArrayInner {
                    errors: Default::default(),
                    syntax: None,
                    value_syntax: None,
                    annotations,
                    items: items.into(),
                }
                .wrap()
                .into()
            }
            Value::Object(value::Object { value, .. }) => {
                let mut entries = Entries::default();
                for (k, v) in value {
                    entries.add(k.into(), v.into());
                }
                ObjectInner {
                    errors: Default::default(),
                    syntax: None,
                    value_syntax: None,
                    annotations,
                    entries: entries.into(),
                }
                .wrap()
                .into()
            }
        }
    }
}

impl From<&Value> for Node {
    fn from(value: &Value) -> Self {
        Self::from_value(value)
    }
}

fn from_value_annotaions(annotations: &IndexMap<String, PlainValue>) -> Option<Annotations> {
    if annotations.is_empty() {
        return None;
    }
    let mut entries: Entries = Default::default();
    for (k, v) in annotations {
        let v = Value::from(v.clone());
        entries.add(k.into(), Node::from_value(&v))
    }
    todo!()
}
