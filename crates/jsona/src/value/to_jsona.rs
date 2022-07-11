use super::*;
use std::fmt::{Display, Result};

impl Value {
    pub fn to_jsona(&self) -> String {
        let mut s = String::new();
        self.to_jsona_fmt(&mut s).unwrap();
        s
    }

    pub fn to_jsona_fmt(&self, f: &mut impl Write) -> Result {
        self.to_jsona_impl(f, false)
    }

    pub fn to_jsona_impl(&self, f: &mut impl Write, comma: bool) -> Result {
        match self {
            Value::Null(Null { annotations, .. }) => {
                f.write_str("null")?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations)?;
            }
            Value::Bool(Bool { value, annotations }) => {
                write!(f, "{}", value)?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations)?;
            }
            Value::Integer(Integer { value, annotations }) => {
                write!(f, "{}", value)?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations)?;
            }
            Value::Float(Float { value, annotations }) => {
                write!(f, "{}", value)?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations)?;
            }
            Value::Str(Str { value, annotations }) => {
                write!(f, "{}", normalize_str(value))?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, annotations)?;
            }
            Value::Array(Array { value, annotations }) => {
                f.write_char('[')?;
                write_annotations(f, annotations)?;
                let len = value.len();
                for (i, item) in value.iter().enumerate() {
                    item.to_jsona_impl(f, i < len - 1)?;
                }
                f.write_char(']')?;
                if comma {
                    f.write_char(',')?;
                }
            }
            Value::Object(Object { value, annotations }) => {
                f.write_char('{')?;
                write_annotations(f, annotations)?;
                let len = value.len();
                for (i, (k, v)) in value.iter().enumerate() {
                    write!(f, "{}:", normalize_str(k))?;
                    v.to_jsona_impl(f, i < len - 1)?;
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
        self.to_jsona_impl(f, false)
    }
}

fn write_annotations(f: &mut impl Write, annotations: &IndexMap<String, Value>) -> Result {
    for (k, v) in annotations {
        if v.is_null() {
            write!(f, "@{}(null)", k)?;
        } else {
            write!(f, "@{}({})", k, v)?;
        }
    }
    Ok(())
}

fn normalize_str(s: &str) -> String {
    if need_quote(s) {
        format!("\"{}\"", s.escape_debug())
    } else {
        s.to_string()
    }
}

fn need_quote(s: &str) -> bool {
    for c in s.chars() {
        if c.is_alphanumeric() {
            continue;
        }
        return true;
    }
    false
}
