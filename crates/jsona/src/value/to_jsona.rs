use crate::util::quote::{check_quote, quote};

use super::*;
use std::fmt::{Display, Result};

impl Value {
    pub fn to_jsona(&self, inline: bool) -> String {
        let mut s = String::new();
        self.to_jsona_fmt(&mut s, inline).unwrap();
        s
    }

    pub fn to_jsona_fmt(&self, f: &mut impl Write, inline: bool) -> Result {
        self.to_jsona_impl(f, inline, 0, false)
    }

    pub fn to_jsona_impl(
        &self,
        f: &mut impl Write,
        inline: bool,
        level: usize,
        comma: bool,
    ) -> Result {
        match self {
            Value::Null(Null { annotations, .. }) => {
                f.write_str("null")?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations, inline, level)?;
            }
            Value::Bool(Bool { value, annotations }) => {
                write!(f, "{}", value)?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations, inline, level)?;
            }
            Value::Integer(Integer { value, annotations }) => {
                write!(f, "{}", value)?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations, inline, level)?;
            }
            Value::Float(Float { value, annotations }) => {
                write!(f, "{}", value)?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations, inline, level)?;
            }
            Value::Str(Str { value, annotations }) => {
                let quote_type = check_quote(value);
                write!(f, "{}", quote(value, quote_type.quote(inline)))?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations, inline, level)?;
            }
            Value::Array(Array { value, annotations }) => {
                f.write_char('[')?;
                write_annotations(f, annotations, inline, level + 1)?;
                let len = value.len();
                for (i, item) in value.iter().enumerate() {
                    if !inline {
                        f.write_char('\n')?;
                        write_ident(f, level + 1)?;
                    }
                    item.to_jsona_impl(f, inline, level + 1, i < len - 1)?;
                }
                if !inline {
                    f.write_char('\n')?;
                    write_ident(f, level)?;
                }
                f.write_char(']')?;
                if comma {
                    f.write_char(',')?;
                }
            }
            Value::Object(Object { value, annotations }) => {
                f.write_char('{')?;
                write_annotations(f, annotations, inline, level + 1)?;
                let len = value.len();
                for (i, (k, v)) in value.iter().enumerate() {
                    if !inline {
                        f.write_char('\n')?;
                        write_ident(f, level + 1)?;
                    }
                    let quote_type = check_quote(k);
                    write!(f, "{}:", quote(k, quote_type.ident()))?;
                    if !inline {
                        f.write_char(' ')?;
                    }
                    v.to_jsona_impl(f, inline, level + 1, i < len - 1)?;
                }
                if !inline {
                    f.write_char('\n')?;
                    write_ident(f, level)?;
                }
                f.write_char('}')?;
                if comma {
                    f.write_char(',')?;
                }
            }
        }
        Ok(())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.to_jsona_impl(f, true, 0, false)
    }
}

fn write_annotations(
    f: &mut impl Write,
    annotations: &IndexMap<String, AnnotationValue>,
    inline: bool,
    level: usize,
) -> Result {
    if annotations.is_empty() {
        return Ok(());
    }
    let len = annotations.len();
    if inline {
        for (i, (k, v)) in annotations.iter().enumerate() {
            if v.is_null() {
                write!(f, "@{}", k)?;
                if !(level == 0 && i == len - 1) {
                    f.write_char(' ')?;
                }
            } else {
                write!(f, "@{}({})", k, Value::from(v.clone()))?;
            }
        }
    } else {
        f.write_char('\n')?;
        for (i, (k, v)) in annotations.iter().enumerate() {
            write_ident(f, level)?;
            if v.is_null() {
                write!(f, "@{}", k)?;
            } else {
                write!(f, "@{}(", k)?;
                Value::from(v.clone()).to_jsona_impl(f, inline, level, false)?;
                f.write_str(")")?;
            }
            if i < len - 1 {
                f.write_char('\n')?;
            }
        }
    }
    Ok(())
}

fn write_ident(f: &mut impl Write, level: usize) -> Result {
    if level > 0 {
        write!(f, "{}", "  ".repeat(level))?;
    }
    Ok(())
}
