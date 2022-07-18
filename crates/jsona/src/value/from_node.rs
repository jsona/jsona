use super::*;
use crate::dom::{DomNode, Node};

impl From<&Node> for Value {
    fn from(node: &Node) -> Self {
        let mut annotations: IndexMap<String, PlainValue> = Default::default();
        if let Some(node_annotations) = node.annotations() {
            for (k, v) in node_annotations.entries().read().iter() {
                annotations.insert(k.value().to_string(), v.into());
            }
        }
        match node {
            Node::Invalid(_) | Node::Null(_) => Null { annotations }.into(),
            Node::Bool(v) => Bool {
                value: v.value(),
                annotations,
            }
            .into(),
            Node::Number(v) => Number {
                value: v.value(),
                annotations,
            }
            .into(),
            Node::Str(v) => Str {
                value: v.value().to_string(),
                annotations,
            }
            .into(),
            Node::Array(v) => {
                let value = v.items().read().iter().map(|v| v.into()).collect();
                Array { items: value, annotations }.into()
            }
            Node::Object(v) => {
                let value = v
                    .entries()
                    .read()
                    .iter()
                    .map(|(k, v)| (k.value().to_string(), v.into()))
                    .collect();
                Object { properties: value, annotations }.into()
            }
        }
    }
}

impl From<&Node> for PlainValue {
    fn from(node: &Node) -> Self {
        match node {
            Node::Invalid(_) | Node::Null(_) => PlainValue::Null,
            Node::Bool(v) => PlainValue::Bool(v.value()),
            Node::Number(v) => PlainValue::Number(v.value()),
            Node::Str(v) => PlainValue::Str(v.value().to_string()),
            Node::Array(v) => {
                let value = v.items().read().iter().map(|v| v.into()).collect();
                PlainValue::Array(value)
            }
            Node::Object(v) => {
                let value = v
                    .entries()
                    .read()
                    .iter()
                    .map(|(k, v)| (k.value().to_string(), v.into()))
                    .collect();
                PlainValue::Object(value)
            }
        }
    }
}
