use serde_json::{json, Value};

use super::*;

impl Node {
    pub fn from_plain_json(data: &Value) -> Self {
        let annotations = None;
        match data {
            Value::Null => NullInner {
                annotations: None,
                ..Default::default()
            }
            .wrap()
            .into(),
            Value::Bool(v) => BoolInner {
                errors: Default::default(),
                syntax: None,
                node_syntax: None,
                annotations,
                value: (*v).into(),
            }
            .wrap()
            .into(),
            Value::Number(v) => {
                let repr = if v.is_f64() {
                    NumberRepr::Float
                } else {
                    NumberRepr::Dec
                };
                NumberInner {
                    errors: Default::default(),
                    syntax: None,
                    node_syntax: None,
                    annotations,
                    repr,
                    value: v.clone().into(),
                }
                .wrap()
                .into()
            }
            Value::String(v) => StringInner {
                errors: Default::default(),
                syntax: None,
                node_syntax: None,
                annotations,
                repr: StringRepr::Double,
                value: v.clone().into(),
            }
            .wrap()
            .into(),
            Value::Array(items) => {
                let items: Vec<Node> = items.iter().map(Node::from_plain_json).collect();
                ArrayInner {
                    errors: Default::default(),
                    syntax: None,
                    node_syntax: None,
                    annotations,
                    items: items.into(),
                }
                .wrap()
                .into()
            }
            Value::Object(properties) => {
                let mut props = Map::default();
                for (k, v) in properties {
                    props.add(Key::property(k), Node::from_plain_json(v));
                }
                ObjectInner {
                    errors: Default::default(),
                    syntax: None,
                    node_syntax: None,
                    annotations,
                    properties: props.into(),
                }
                .wrap()
                .into()
            }
        }
    }

    pub fn from_json(data: &Value) -> Self {
        let value = match data.get("value") {
            Some(v) => v,
            None => {
                return NullInner {
                    errors: Default::default(),
                    node_syntax: None,
                    syntax: None,
                    annotations: None,
                }
                .wrap()
                .into()
            }
        };
        let annotations = data
            .get("annotations")
            .and_then(|v| v.as_object())
            .map(|m| {
                let mut members = Map::default();
                for (k, v) in m {
                    members.add(Key::annotation(k), Node::from_plain_json(v))
                }
                AnnotationsInner {
                    errors: Default::default(),
                    members: members.into(),
                }
                .into()
            });
        match value {
            Value::Null => NullInner {
                annotations,
                ..Default::default()
            }
            .wrap()
            .into(),
            Value::Bool(v) => BoolInner {
                errors: Default::default(),
                syntax: None,
                node_syntax: None,
                annotations,
                value: (*v).into(),
            }
            .wrap()
            .into(),
            Value::Number(v) => {
                let repr = if v.is_f64() {
                    NumberRepr::Float
                } else {
                    NumberRepr::Dec
                };
                NumberInner {
                    errors: Default::default(),
                    syntax: None,
                    node_syntax: None,
                    annotations: None,
                    repr,
                    value: v.clone().into(),
                }
                .wrap()
                .into()
            }
            Value::String(v) => StringInner {
                errors: Default::default(),
                syntax: None,
                node_syntax: None,
                annotations: None,
                repr: StringRepr::Double,
                value: v.clone().into(),
            }
            .wrap()
            .into(),
            Value::Array(items) => {
                let items: Vec<Node> = items.iter().map(Node::from_json).collect();
                ArrayInner {
                    errors: Default::default(),
                    syntax: None,
                    node_syntax: None,
                    annotations: None,
                    items: items.into(),
                }
                .wrap()
                .into()
            }
            Value::Object(properties) => {
                let mut props = Map::default();
                for (k, v) in properties {
                    props.add(Key::property(k), Node::from_json(v));
                }
                ObjectInner {
                    errors: Default::default(),
                    syntax: None,
                    node_syntax: None,
                    annotations: None,
                    properties: props.into(),
                }
                .wrap()
                .into()
            }
        }
    }

    pub fn to_plain_json(&self) -> Value {
        match self {
            Node::Null(_) => Value::Null,
            Node::Bool(v) => v.value().into(),
            Node::Number(v) => v.value().clone().into(),
            Node::String(v) => v.value().into(),
            Node::Array(v) => {
                Value::Array(v.value().read().iter().map(|v| v.to_plain_json()).collect())
            }
            Node::Object(v) => Value::Object(
                v.value()
                    .read()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_plain_json()))
                    .collect(),
            ),
        }
    }

    pub fn to_json(&self) -> Value {
        let annotations = self.annotations().map(|a| {
            Value::Object(
                a.value()
                    .read()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_plain_json()))
                    .collect(),
            )
        });
        match self {
            Node::Null(_) => match annotations {
                Some(annotations) => {
                    json!({
                        "value": null,
                        "annotations": annotations
                    })
                }
                None => {
                    json!({
                        "value": null,
                    })
                }
            },
            Node::Bool(v) => match annotations {
                Some(annotations) => {
                    json!({
                        "value": v.value(),
                        "annotations": annotations
                    })
                }
                None => {
                    json!({
                        "value": v.value(),
                    })
                }
            },
            Node::Number(v) => match annotations {
                Some(annotations) => {
                    json!({
                        "value": v.value(),
                        "annotations": annotations
                    })
                }
                None => {
                    json!({
                        "value": v.value(),
                    })
                }
            },
            Node::String(v) => match annotations {
                Some(annotations) => {
                    json!({
                        "value": v.value(),
                        "annotations": annotations
                    })
                }
                None => {
                    json!({
                        "value": v.value(),
                    })
                }
            },
            Node::Array(v) => {
                let value = Value::Array(v.value().read().iter().map(|v| v.to_json()).collect());
                match annotations {
                    Some(annotations) => {
                        json!({
                            "value": value,
                            "annotations": annotations
                        })
                    }
                    None => {
                        json!({
                            "value": value,
                        })
                    }
                }
            }
            Node::Object(v) => {
                let value = Value::Object(
                    v.value()
                        .read()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_json()))
                        .collect(),
                );
                match annotations {
                    Some(annotations) => {
                        json!({
                            "value": value,
                            "annotations": annotations
                        })
                    }
                    None => {
                        json!({
                            "value": value,
                        })
                    }
                }
            }
        }
    }
}
