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
                write!(f, "{}", normalize_str(value))?;
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
                    write!(f, "{}:", normalize_str(k))?;
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
    annotations: &IndexMap<String, Value>,
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
                write!(f, "@{}({})", k, v)?;
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
                v.to_jsona_impl(f, inline, level, false)?;
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
