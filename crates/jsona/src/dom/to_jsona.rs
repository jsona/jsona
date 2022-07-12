use super::*;
use crate::util::quote::{check_quote, quote};
use std::fmt::{Display, Formatter, Result, Write};

impl Node {
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
            Node::Null(_) => {
                f.write_str("null")?;
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, self.annotations(), inline, level)?;
            }
            Node::Bool(v) => {
                match self.syntax() {
                    Some(syntax) => {
                        write!(f, "{}", syntax)?;
                    }
                    None => {
                        write!(f, "{}", v.value())?;
                    }
                }
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, self.annotations(), inline, level)?;
            }
            Node::Integer(v) => {
                match self.syntax() {
                    Some(syntax) => {
                        write!(f, "{}", syntax)?;
                    }
                    None => {
                        write!(f, "{}", v.value())?;
                    }
                }
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, v.annotations(), inline, level)?;
            }
            Node::Float(v) => {
                match self.syntax() {
                    Some(syntax) => {
                        write!(f, "{}", syntax)?;
                    }
                    None => {
                        write!(f, "{}", v.value())?;
                    }
                }
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, v.annotations(), inline, level)?;
            }
            Node::Str(v) => {
                match self.syntax() {
                    Some(syntax) => {
                        write!(f, "{}", syntax)?;
                    }
                    None => {
                        let value = v.value();
                        let quote_type = check_quote(value);
                        write!(f, "{}", quote(value, quote_type.quote(inline)))?;
                    }
                }
                if comma {
                    f.write_char(',')?;
                }
                write_annotations(f, v.annotations(), inline, level)?;
            }
            Node::Array(v) => {
                f.write_char('[')?;
                write_annotations(f, v.annotations(), inline, level + 1)?;
                let value = v.items().read();
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
            Node::Object(v) => {
                f.write_char('{')?;
                write_annotations(f, v.annotations(), inline, level + 1)?;
                let value = v.entries().read();
                let len = value.len();
                for (i, (k, v)) in value.iter().enumerate() {
                    if !inline {
                        f.write_char('\n')?;
                        write_ident(f, level + 1)?;
                    }
                    match k.syntax() {
                        Some(syntax) => {
                            write!(f, "{}:", syntax)?;
                        }
                        None => {
                            let value = k.value();
                            let quote_type = check_quote(value);
                            write!(f, "{}:", quote(value, quote_type.ident()))?;
                        }
                    }
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
            Node::Invalid(_) => {}
        }
        Ok(())
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.to_jsona_impl(f, true, 0, false)
    }
}

fn write_annotations(
    f: &mut impl Write,
    annotations: &Option<Annotations>,
    inline: bool,
    level: usize,
) -> Result {
    match annotations {
        Some(annotations) => {
            let annotations = annotations.entries().read();
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
        }
        None => {}
    }
    Ok(())
}

fn write_ident(f: &mut impl Write, level: usize) -> Result {
    if level > 0 {
        write!(f, "{}", "  ".repeat(level))?;
    }
    Ok(())
}
