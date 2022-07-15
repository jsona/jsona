use crate::formatter::{Scope, ScopeKind};
use crate::util::quote::quote;

use super::*;
use std::fmt::{Display, Result};

impl Value {
    pub fn to_jsona(&self) -> String {
        let scope = Scope::default();
        write_value(scope.clone(), self);
        if scope.is_last_char(',') {
            scope.remove_last_char();
        }
        scope.read()
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.to_jsona())
    }
}

fn write_value(scope: Scope, value: &Value) {
    match value {
        Value::Null(Null { annotations, .. }) => {
            write_scalar(scope, "null", annotations);
        }
        Value::Bool(Bool { value, annotations }) => {
            write_scalar(scope, value, annotations);
        }
        Value::Integer(Integer { value, annotations }) => {
            write_scalar(scope, value, annotations);
        }
        Value::Float(Float { value, annotations }) => {
            write_scalar(scope, value, annotations);
        }
        Value::Str(Str { value, annotations }) => {
            let value = quote(value, true);
            write_scalar(scope, value, annotations);
        }
        Value::Array(Array { value, annotations }) => {
            if scope.kind == ScopeKind::Array {
                scope.write_ident();
                scope.write("[");
            } else {
                scope.write("[");
            }
            let scope = scope.enter(ScopeKind::Array);
            write_annotations(scope.clone(), annotations);
            scope.newline();
            for item in value {
                write_value(scope.clone(), item);
                scope.newline();
            }
            let scope = scope.exit();
            scope.write_ident();
            scope.write("],");
        }
        Value::Object(Object { value, annotations }) => {
            if scope.kind == ScopeKind::Array {
                scope.write_ident();
                scope.write("{");
            } else {
                scope.write("{");
            }
            let scope = scope.enter(ScopeKind::Object);
            write_annotations(scope.clone(), annotations);
            scope.newline();
            for (k, v) in value {
                scope.write_ident();
                scope.write(format!("{}: ", k));
                write_value(scope.clone(), v);
                scope.newline();
            }
            let scope = scope.exit();
            scope.write_ident();
            scope.write("},");
        }
    }
}

fn write_scalar<T: Display>(
    scope: Scope,
    value: T,
    annotations: &IndexMap<String, AnnotationValue>,
) {
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

fn write_annotations(scope: Scope, annotations: &IndexMap<String, AnnotationValue>) {
    for (key, value) in annotations {
        match value {
            AnnotationValue::Null(_) => {
                scope.write(format!(" @{}", key));
            }
            AnnotationValue::Bool(inner) => write_scalar_annotaion(scope.clone(), key, inner),
            AnnotationValue::Integer(inner) => write_scalar_annotaion(scope.clone(), key, inner),
            AnnotationValue::Float(inner) => write_scalar_annotaion(scope.clone(), key, inner),
            AnnotationValue::Str(inner) => write_scalar_annotaion(scope.clone(), key, inner),
            AnnotationValue::Array(_) | AnnotationValue::Object(_) => {
                scope.write("\n");
                scope.write_ident();
                scope.write(format!("@{}(", key));
                let value: Value = value.clone().into();
                write_value(scope.clone(), &value);
                scope.exit();
                if scope.is_last_char(',') {
                    scope.remove_last_char();
                }
                scope.write(")");
            }
        }
    }
    scope.newline();
}

fn write_scalar_annotaion<T: Display>(scope: Scope, key: &str, value: T) {
    scope.write(format!(" @{}({})", key, value));
}
