use std::error::Error;
use std::fmt::{self, Display};

use crate::value::{Amap, Object, Value};

#[derive(Copy, Clone, Debug)]
pub enum EmitError {
    FmtError(fmt::Error),
}

impl Error for EmitError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EmitError::FmtError(ref err) => Display::fmt(err, formatter),
        }
    }
}

impl From<fmt::Error> for EmitError {
    fn from(f: fmt::Error) -> Self {
        EmitError::FmtError(f)
    }
}

pub type EmitResult = Result<(), EmitError>;
pub struct Emitter<'a> {
    writer: &'a mut dyn fmt::Write,
    indent: usize,
    level: usize,
}

impl<'a> Emitter<'a> {
    pub fn new(writer: &'a mut dyn fmt::Write) -> Self {
        Self {
            writer,
            indent: 2,
            level: 0,
        }
    }
    pub fn set_indent(&mut self, indent: usize) {
        self.indent = indent;
    }
    pub fn emit(&mut self, data: &(Value, Option<Amap>)) -> EmitResult {
        self.emit_doc_annotations(&data.1)?;
        writeln!(self.writer)?;
        self.emit_value(&data.0)?;
        Ok(())
    }
    pub fn emit_doc_annotations(&mut self, annotations: &Option<Amap>) -> EmitResult {
        if let Some(a) = annotations {
            for (name, args) in a.iter() {
                write!(self.writer, "@{}", name)?;
                let len = args.len();
                if len > 0 {
                    write!(self.writer, "(")?;
                    for (i, arg) in args.iter().enumerate() {
                        self.write_string(arg.as_str(), false)?;
                        if i < len - 1 {
                            write!(self.writer, ",")?;
                        }
                    }
                    write!(self.writer, ")")?;
                }
                writeln!(self.writer)?;
            }
        }
        Ok(())
    }
    fn emit_annotations(&mut self, annotations: &Option<Amap>) -> EmitResult {
        if let Some(a) = annotations {
            for (name, args) in a.iter() {
                write!(self.writer, " @{}", name)?;
                let len = args.len();
                if len > 0 {
                    write!(self.writer, "(")?;
                    for (i, arg) in args.iter().enumerate() {
                        self.write_string(arg.as_str(), false)?;
                        if i < len - 1 {
                            write!(self.writer, ",")?;
                        }
                    }
                    write!(self.writer, ")")?;
                }
            }
        }
        Ok(())
    }
    pub fn emit_value(&mut self, node: &Value) -> EmitResult {
        self.emit_node(node)?;
        if node.is_scalar() {
            self.emit_annotations(node.get_annotations())?;
        }
        Ok(())
    }
    fn emit_node(&mut self, node: &Value) -> EmitResult {
        match *node {
            Value::Null(_) => {
                self.writer.write_str("null")?;
                Ok(())
            }
            Value::Boolean(b, _) => {
                if b {
                    self.writer.write_str("true")?;
                } else {
                    self.writer.write_str("false")?;
                }
                Ok(())
            }
            Value::Integer(i, _) => {
                write!(self.writer, "{}", i)?;
                Ok(())
            }
            Value::Float(f, _) => {
                write!(self.writer, "{}", f)?;
                Ok(())
            }
            Value::String(ref s, _) => {
                self.write_string(s.as_str(), true)?;
                Ok(())
            }
            Value::Array(ref v, ref a) => {
                self.emit_array(v, a)?;
                Ok(())
            }
            Value::Object(ref o, ref a) => {
                self.emit_object(o, a)?;
                Ok(())
            }
            Value::BadValue(..) => Ok(()),
        }
    }
    fn write_indent(&mut self) -> EmitResult {
        for _ in 0..self.level {
            for _ in 0..self.indent {
                write!(self.writer, " ")?;
            }
        }
        Ok(())
    }
    fn emit_array(&mut self, v: &[Value], a: &Option<Amap>) -> EmitResult {
        if v.is_empty() {
            write!(self.writer, "[]")?;
        } else {
            write!(self.writer, "[")?;
            self.emit_annotations(a)?;
            writeln!(self.writer)?;
            self.level += 1;
            for x in v.iter() {
                self.write_indent()?;
                self.emit_node(x)?;
                write!(self.writer, ",")?;
                if x.is_scalar() {
                    self.emit_annotations(x.get_annotations())?;
                }
                writeln!(self.writer)?;
            }
            self.level -= 1;
            self.write_indent()?;
            write!(self.writer, "]")?;
        }
        Ok(())
    }
    fn emit_object(&mut self, o: &Object, a: &Option<Amap>) -> EmitResult {
        if o.is_empty() {
            self.writer.write_str("{}")?;
        } else {
            write!(self.writer, "{{")?;
            self.emit_annotations(a)?;
            writeln!(self.writer)?;
            self.level += 1;
            for (k, v) in o.iter() {
                self.write_indent()?;
                self.write_string(k.as_str(), false)?;
                write!(self.writer, ": ")?;
                self.emit_node(v)?;
                write!(self.writer, ",")?;
                if v.is_scalar() {
                    self.emit_annotations(v.get_annotations())?;
                }
                writeln!(self.writer)?;
            }
            self.level -= 1;
            self.write_indent()?;
            write!(self.writer, "}}")?;
        }
        Ok(())
    }
    fn write_string(&mut self, s: &str, quota: bool) -> EmitResult {
        if quota || need_quotes(s) {
            escape_str(self.writer, s)?;
        } else {
            write!(self.writer, "{}", s)?;
        }
        Ok(())
    }
}

/// Check if the string requires quoting.
/// Strings starting with any of the following characters must be quoted.
/// :, &, *, ?, |, -, <, >, =, !, %, @
/// Strings containing any of the following characters must be quoted.
/// {, }, [, ], ,, #, `
///
/// If the string contains any of the following control characters, it must be escaped with double quotes:
/// \0, \x01, \x02, \x03, \x04, \x05, \x06, \a, \b, \t, \n, \v, \f, \r, \x0e, \x0f, \x10, \x11, \x12, \x13, \x14, \x15, \x16, \x17, \x18, \x19, \x1a, \e, \x1c, \x1d, \x1e, \x1f, \N, \_, \L, \P
///
/// Finally, there are other cases when the strings must be quoted, no matter if you're using single or double quotes:
/// * When the string is true or false (otherwise, it would be treated as a boolean value);
/// * When the string is null or ~ (otherwise, it would be considered as a null value);
/// * When the string looks like a number, such as integers (e.g. 2, 14, etc.), floats (e.g. 2.6, 14.9) and exponential numbers (e.g. 12e7, etc.) (otherwise, it would be treated as a numeric value);
/// * When the string looks like a date (e.g. 2014-12-31) (otherwise it would be automatically converted into a Unix timestamp).
fn need_quotes(string: &str) -> bool {
    fn need_quotes_spaces(string: &str) -> bool {
        string.starts_with(' ') || string.ends_with(' ')
    }

    string == ""
        || need_quotes_spaces(string)
        || string.starts_with(|character: char| match character {
            '&' | '*' | '?' | '|' | '-' | '<' | '>' | '=' | '!' | '%' | '@' => true,
            _ => false,
        })
        || string.contains(|character: char| match character {
            ':'
            | '{'
            | '}'
            | '['
            | ']'
            | ','
            | '#'
            | '`'
            | '\"'
            | '\''
            | '\\'
            | '\0'..='\x06'
            | '\t'
            | '\n'
            | '\r'
            | '\x0e'..='\x1a'
            | '\x1c'..='\x1f' => true,
            _ => false,
        })
        || string.starts_with('.')
        || string.starts_with("0x")
        || string.parse::<i64>().is_ok()
        || string.parse::<f64>().is_ok()
}

fn escape_str(wr: &mut dyn fmt::Write, v: &str) -> Result<(), fmt::Error> {
    wr.write_str("\"")?;

    let mut start = 0;

    for (i, byte) in v.bytes().enumerate() {
        let escaped = match byte {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\x00' => "\\u0000",
            b'\x01' => "\\u0001",
            b'\x02' => "\\u0002",
            b'\x03' => "\\u0003",
            b'\x04' => "\\u0004",
            b'\x05' => "\\u0005",
            b'\x06' => "\\u0006",
            b'\x07' => "\\u0007",
            b'\x08' => "\\b",
            b'\t' => "\\t",
            b'\n' => "\\n",
            b'\x0b' => "\\u000b",
            b'\x0c' => "\\f",
            b'\r' => "\\r",
            b'\x0e' => "\\u000e",
            b'\x0f' => "\\u000f",
            b'\x10' => "\\u0010",
            b'\x11' => "\\u0011",
            b'\x12' => "\\u0012",
            b'\x13' => "\\u0013",
            b'\x14' => "\\u0014",
            b'\x15' => "\\u0015",
            b'\x16' => "\\u0016",
            b'\x17' => "\\u0017",
            b'\x18' => "\\u0018",
            b'\x19' => "\\u0019",
            b'\x1a' => "\\u001a",
            b'\x1b' => "\\u001b",
            b'\x1c' => "\\u001c",
            b'\x1d' => "\\u001d",
            b'\x1e' => "\\u001e",
            b'\x1f' => "\\u001f",
            b'\x7f' => "\\u007f",
            _ => continue,
        };

        if start < i {
            wr.write_str(&v[start..i])?;
        }

        wr.write_str(escaped)?;

        start = i + 1;
    }

    if start != v.len() {
        wr.write_str(&v[start..])?;
    }

    wr.write_str("\"")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::Loader;

    #[test]
    fn test_emit() {
        let s = r##"
@abc
@def(a, b)
/*
multi-line comments
*/

{
    a: null,
    b: 'say "hello"',
    c: true,
    m: "it's awesome",
    h: -3.13,
    d: [ @array
        "abc", @upper
        "def",
    ],
    o: { a:3, b: 4 },
    // This is comments
    g: { @object
        a: 3,
        b: 4,
        c: 5,
    },
    x: 0x1b,
    y: 3.2 @optional @xxg(a, b)
}
        "##;

        let t = r##"@abc
@def(a,b)

{
  a: null,
  b: "say \"hello\"",
  c: true,
  m: "it's awesome",
  h: -3.13,
  d: [ @array
    "abc", @upper
    "def",
  ],
  o: {
    a: 3,
    b: 4,
  },
  g: { @object
    a: 3,
    b: 4,
    c: 5,
  },
  x: 27,
  y: 3.2, @optional @xxg(a,b)
}"##;
        let result = Loader::load_from_str(s).unwrap();
        let mut writer = String::new();
        {
            let mut emitter = Emitter::new(&mut writer);
            emitter.emit(&result).unwrap();
        }
        assert_eq!(writer, t);
    }
}
