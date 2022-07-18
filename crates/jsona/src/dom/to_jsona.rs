use super::*;
use crate::formatter::{Scope, ScopeKind};
use std::fmt::{Display, Formatter, Result};

impl Node {
    pub fn to_jsona(&self) -> String {
        let scope = Scope::default();
        write_value(scope.clone(), self);
        if scope.is_last_char(',') {
            scope.remove_last_char();
        }
        scope.output()
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.to_jsona())
    }
}

fn write_value(scope: Scope, value: &Node) {
    match value {
        Node::Null(_) | Node::Bool(_) | Node::Number(_) | Node::Str(_) => {
            if let Some(text) = value.jsona_text() {
                write_scalar(scope, text, value.annotations());
            }
        }
        Node::Array(v) => {
            if scope.kind == ScopeKind::Array {
                scope.write_ident();
                scope.write("[");
            } else {
                scope.write("[");
            }
            let scope = scope.enter(ScopeKind::Array);
            write_annotations(scope.clone(), v.annotations());
            scope.newline();
            let value = v.value().read();
            for item in value.iter() {
                write_value(scope.clone(), item);
                scope.newline();
            }
            let scope = scope.exit();
            scope.write_ident();
            scope.write("],");
        }
        Node::Object(v) => {
            if scope.kind == ScopeKind::Array {
                scope.write_ident();
                scope.write("{");
            } else {
                scope.write("{");
            }
            let scope = scope.enter(ScopeKind::Object);
            write_annotations(scope.clone(), v.annotations());
            scope.newline();
            let value = v.value().read();
            for (k, v) in value.iter() {
                scope.write_ident();
                scope.write(format!("{}: ", k));
                write_value(scope.clone(), v);
                scope.newline();
            }
            let scope = scope.exit();
            scope.write_ident();
            scope.write("},");
        }
        Node::Invalid(_) => {}
    }
}

fn write_scalar<T: Display>(scope: Scope, value: T, annotations: Option<&Annotations>) {
    if scope.kind == ScopeKind::Array {
        scope.write_ident();
        scope.write(format!("{},", value));
    } else if scope.kind == ScopeKind::Object {
        scope.write(format!("{},", value));
    } else {
        scope.write(value.to_string());
    }
    write_annotations(scope, annotations);
}

fn write_annotations(scope: Scope, annotations: Option<&Annotations>) {
    match annotations {
        Some(annotations) => {
            let annotations = annotations.value().read();
            for (key, value) in annotations.iter() {
                match value {
                    Node::Null(_) | Node::Invalid(_) => {
                        scope.write(format!(" @{}", key));
                    }
                    Node::Bool(_) | Node::Number(_) | Node::Str(_) => {
                        if let Some(text) = value.jsona_text() {
                            write_scalar_annotaion(scope.clone(), key, text);
                        }
                    }
                    Node::Array(_) | Node::Object(_) => {
                        scope.write("\n");
                        scope.write_ident();
                        scope.write(format!("@{}(", key));
                        write_value(scope.clone(), value);
                        scope.exit();
                        if scope.is_last_char(',') {
                            scope.remove_last_char();
                        }
                        scope.write(")");
                    }
                }
            }
        }
        None => {}
    }
    scope.newline();
}

fn write_scalar_annotaion<T: Display>(scope: Scope, key: &Key, value: T) {
    scope.write(format!(" @{}({})", key, value));
}
