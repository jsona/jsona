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
