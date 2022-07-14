use crate::formatter::{Scope, ScopeKind};
use crate::util::quote::{check_quote, quote};

use super::*;
use std::{
    fmt::{Display, Result},
    rc::Rc,
};

impl Value {
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
            let quote_type = check_quote(value);
            let value = quote(value, quote_type.quote(false));
            write_scalar(scope, value, annotations);
        }
        Value::Array(Array { value, annotations }) => {
            if scope.kind == ScopeKind::Array {
                scope.write_with_ident("[");
            } else {
                scope.write("[");
            }
            let scope = scope.enter(ScopeKind::Array);
            write_annotations(scope.clone(), annotations);
            scope.maybe_newline();
            for item in value {
                write_value(scope.clone(), item);
                scope.maybe_newline();
            }
            let scope = scope.exit();
            scope.write_with_ident("],");
        }
        Value::Object(Object { value, annotations }) => {
            if scope.kind == ScopeKind::Array {
                scope.write_with_ident("{");
            } else {
                scope.write("{");
            }
            let scope = scope.enter(ScopeKind::Object);
            write_annotations(scope.clone(), annotations);
            scope.maybe_newline();
            for (k, v) in value {
                scope.write_with_ident(format!("{}: ", k));
                write_value(scope.clone(), v);
                scope.maybe_newline();
            }
            let scope = scope.exit();
            scope.write_with_ident("},");
        }
    }
}

fn write_scalar<T: Display>(
    scope: Scope,
    value: T,
    annotations: &IndexMap<String, AnnotationValue>,
) {
    if scope.kind == ScopeKind::Array {
        scope.write_with_ident(format!("{},", value));
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
                scope.write_with_ident(format!("@{}(", key));
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
    scope.maybe_newline();
}

fn write_scalar_annotaion<T: Display>(scope: Scope, key: &str, value: T) {
    scope.write(format!(" @{}({})", key, value));
}
