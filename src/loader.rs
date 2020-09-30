use indexmap::IndexMap;

use crate::lexer::Position;
use crate::parser::{Event, EventReceiver, ParseResult, Parser};
use crate::value::{Amap, Value};

pub struct Loader {
    value_stack: Vec<Value>,
    key_stack: Vec<String>,
    annotations: Option<Amap>,
}

impl Loader {
    pub fn load_from_str(input: &str) -> ParseResult<(Value, Option<Amap>)> {
        let mut loader = Loader {
            value_stack: Vec::new(),
            key_stack: Vec::new(),
            annotations: None,
        };
        let mut parser = Parser::new(input.chars());
        parser.parse(&mut loader)?;
        Ok((loader.value_stack.pop().unwrap(), loader.annotations))
    }
    fn insert_new_node(&mut self, node: Value) {
        if self.value_stack.is_empty() {
            self.value_stack.push(node);
        } else {
            let parent = self.value_stack.last_mut().unwrap();
            match *parent {
                Value::Array(ref mut v, _) => v.push(node),
                Value::Object(ref mut v, _) => {
                    let mut cur_key = self.key_stack.pop().unwrap();
                    if cur_key.is_empty() {
                        if let Value::String(s, _) = node {
                            cur_key = s;
                        } else {
                            unreachable!()
                        }
                    } else {
                        let key = cur_key;
                        v.insert(key, node);
                        cur_key = String::new();
                    }
                    self.key_stack.push(cur_key);
                }
                _ => unreachable!(),
            }
        }
    }
    fn insert_annotations(&mut self, annotations: Option<Amap>) {
        if self.value_stack.is_empty() {
            self.annotations = annotations;
        } else {
            let parent = self.value_stack.last_mut().unwrap();
            match *parent {
                Value::Array(ref mut v, ref mut a) => {
                    if v.len() > 0 {
                        v.last_mut().unwrap().set_annotations(annotations);
                    } else {
                        *a = annotations;
                    }
                }
                Value::Object(ref mut v, ref mut a) => {
                    if v.len() > 0 {
                        v.get_index_mut(v.len() - 1)
                            .unwrap()
                            .1
                            .set_annotations(annotations);
                    } else {
                        *a = annotations;
                    }
                }
                ref mut v => v.set_annotations(annotations),
            }
        }
    }
}

impl EventReceiver for Loader {
    fn on_event(&mut self, ev: Event, _pos: Position) {
        match ev {
            Event::ArrayStart => {
                self.value_stack.push(Value::Array(Vec::new(), None));
            }
            Event::ArrayStop => {
                let node = self.value_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::ObjectStart => {
                self.key_stack.push(String::new());
                self.value_stack.push(Value::Object(IndexMap::new(), None));
            }
            Event::ObjectStop => {
                self.key_stack.pop().unwrap();
                let node = self.value_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::Null => {
                let node = Value::Null(None);
                self.insert_new_node(node);
            }
            Event::Float(f) => {
                let node = Value::Float(f, None);
                self.insert_new_node(node);
            }
            Event::Integer(i) => {
                let node = Value::Integer(i, None);
                self.insert_new_node(node);
            }
            Event::Boolean(b) => {
                let node = Value::Boolean(b, None);
                self.insert_new_node(node);
            }
            Event::String(s) => {
                let node = Value::String(s, None);
                self.insert_new_node(node);
            }
            Event::Annotations(annotations) => {
                self.insert_annotations(Some(annotations));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    macro_rules! map(
        { $($key:expr => $value:expr),+ } => {
            {
                let mut m = IndexMap::new();
                $(
                    m.insert($key, $value);
                )+
                m
            }
        };
    );

    #[test]
    fn test_load() {
        let s = r##"
@abc
@def(k1 = "a", k2 = "b")
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
    y: 3.2 @optional @xxg(k1 = "a", k2 = "b")
}
        "##;
        let result = Loader::load_from_str(s).unwrap();
        let value = Value::Object(
            map! {
                "a".into() => Value::Null(None),
                "b".into() => Value::String(r#"say "hello""#.into(), None),
                "c".into() => Value::Boolean(true, None),
                "m".into() => Value::String(r#"it's awesome"#.into(), None),
                "h".into() => Value::Float(-3.13, None),
                "d".into() => Value::Array(
                    vec![
                        Value::String("abc".into(), Some(map!{ "upper".into() => IndexMap::new() })),
                        Value::String("def".into(), None),
                    ],
                    Some(map!{ "array".into() => IndexMap::new() })
                ),
                "o".into() => Value::Object(map!{
                    "a".into() => Value::Integer(3, None),
                    "b".into() => Value::Integer(4, None)
                }, None),
                "g".into() => Value::Object(
                    map!{
                        "a".into() => Value::Integer(3, None),
                        "b".into() => Value::Integer(4, None),
                        "c".into() => Value::Integer(5, None)
                    },
                    Some(map!{ "object".into() => IndexMap::new() })
                ),
                "x".into() => Value::Integer(27, None),
                "y".into() => Value::Float(
                    3.2,
                    Some(map!{
                        "optional".into() => IndexMap::new(),
                        "xxg".into() => map!{ "k1".into() => "a".into(), "k2".into() => "b".into() }
                    })
                )
            },
            None,
        );
        let annotations: Amap = map! {
            "abc".into() => IndexMap::new(),
            "def".into() => map!{ "k1".into() => "a".into(), "k2".into() => "b".into() }
        };
        assert_eq!((value, Some(annotations)), result);
    }
}
