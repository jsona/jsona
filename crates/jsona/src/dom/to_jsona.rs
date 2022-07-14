use super::*;
use crate::formatter::{Scope, ScopeKind};
use std::fmt::{Display, Formatter, Result};
use std::rc::Rc;

impl Node {
    pub fn to_jsona(&self) -> String {
        let scope = Scope {
            options: Rc::new(Default::default()),
            level: 0,
            formatted: Default::default(),
            error_ranges: Rc::new(vec![]),
            kind: ScopeKind::Root,
        };
        write_value(scope.clone(), self);
        if scope.is_last_char(',') {
            scope.remove_last_char();
        }
        scope.read()
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.to_jsona())
    }
}

fn write_value(scope: Scope, value: &Node) {
    match value {
        Node::Null(_) | Node::Bool(_) | Node::Integer(_) | Node::Float(_) | Node::Str(_) => {
            if let Some(text) = value.jsona_text() {
                write_scalar(scope, text, value.annotations());
            }
        }
        Node::Array(v) => {
            if scope.kind == ScopeKind::Array {
                scope.write_with_ident("[");
            } else {
                scope.write("[");
            }
            let scope = scope.enter(ScopeKind::Array);
            write_annotations(scope.clone(), v.annotations());
            scope.maybe_newline();
            let value = v.items().read();
            for item in value.iter() {
                write_value(scope.clone(), item);
                scope.maybe_newline();
            }
            let scope = scope.exit();
            scope.write_with_ident("],");
        }
        Node::Object(v) => {
            if scope.kind == ScopeKind::Array {
                scope.write_with_ident("{");
            } else {
                scope.write("{");
            }
            let scope = scope.enter(ScopeKind::Object);
            write_annotations(scope.clone(), v.annotations());
            scope.maybe_newline();
            let value = v.entries().read();
            for (k, v) in value.iter() {
                scope.write_with_ident(format!("{}: ", k));
                write_value(scope.clone(), v);
                scope.maybe_newline();
            }
            let scope = scope.exit();
            scope.write_with_ident("},");
        }
        Node::Invalid(_) => {}
    }
}

fn write_scalar<T: Display>(scope: Scope, value: T, annotations: &Option<Annotations>) {
    if scope.kind == ScopeKind::Array {
        scope.write_with_ident(format!("{},", value));
    } else if scope.kind == ScopeKind::Object {
        scope.write(format!("{},", value));
    } else {
        scope.write(value.to_string());
    }
    write_annotations(scope, annotations);
}

fn write_annotations(scope: Scope, annotations: &Option<Annotations>) {
    match annotations {
        Some(annotations) => {
            let annotations = annotations.entries().read();
            for (key, value) in annotations.iter() {
                match value {
                    Node::Null(_) | Node::Invalid(_) => {
                        scope.write(format!(" @{}", key));
                    }
                    Node::Bool(_) | Node::Integer(_) | Node::Float(_) | Node::Str(_) => {
                        if let Some(text) = value.jsona_text() {
                            write_scalar_annotaion(scope.clone(), key, text);
                        }
                    }
                    Node::Array(_) | Node::Object(_) => {
                        scope.write("\n");
                        scope.write_with_ident(format!("@{}(", key));
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
    scope.maybe_newline();
}

fn write_scalar_annotaion<T: Display>(scope: Scope, key: &Key, value: T) {
    scope.write(format!(" @{}({})", key, value));
}
