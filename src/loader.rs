use indexmap::IndexMap;

use crate::lexer::Position;
use crate::parser::{Event, EventReceiver, ParseResult, Parser};
use crate::value::{Amap, Doc, Value, Key};

pub struct Loader {
    value_stack: Vec<Value>,
    key_stack: Vec<Key>,
    annotations: Option<Amap>,
}

impl Loader {
    pub fn load_from_str(input: &str) -> ParseResult<Doc> {
        let mut loader = Loader {
            value_stack: Vec::new(),
            key_stack: Vec::new(),
            annotations: None,
        };
        let mut parser = Parser::new(input.chars());
        parser.parse(&mut loader)?;
        Ok(Doc {
            value: loader.value_stack.pop().unwrap(),
            annotation: loader.annotations,
        })
    }
    fn insert_new_node(&mut self, node: Value) {
        if self.value_stack.is_empty() {
            self.value_stack.push(node);
        } else {
            let parent = self.value_stack.last_mut().unwrap();
            match *parent {
                Value::Array { ref mut value, .. } => value.push(node),
                Value::Object { ref mut value, .. } => {
                    let mut cur_key = self.key_stack.pop().unwrap();
                    if cur_key.is_empty() {
                        if let Value::String {
                            value,
                            position,
                            ..
                        } = node
                        {
                            cur_key = Key::new(value, position);
                        } else {
                            unreachable!()
                        }
                    } else {
                        let key = cur_key;
                        value.insert(key, node);
                        cur_key = Key::create("")
                    }
                    self.key_stack.push(cur_key);
                }
                _ => unreachable!(),
            }
        }
    }
    fn insert_annotations(&mut self, _annotations: Option<Amap>) {
        if self.value_stack.is_empty() {
            self.annotations = _annotations;
        } else {
            let parent = self.value_stack.last_mut().unwrap();
            match *parent {
                Value::Array {
                    ref mut value,
                    ref mut annotations,
                    ..
                } => {
                    if value.len() > 0 {
                        value.last_mut().unwrap().set_annotations(_annotations);
                    } else {
                        *annotations = _annotations;
                    }
                }
                Value::Object {
                    ref mut value,
                    ref mut annotations,
                    ..
                } => {
                    if value.len() > 0 {
                        value
                            .get_index_mut(value.len() - 1)
                            .unwrap()
                            .1
                            .set_annotations(_annotations);
                    } else {
                        *annotations = _annotations;
                    }
                }
                ref mut value => value.set_annotations(_annotations),
            }
        }
    }
}

impl EventReceiver for Loader {
    fn on_event(&mut self, event: Event, position: Position) {
        match event {
            Event::ArrayStart => {
                self.value_stack.push(Value::Array {
                    value: Vec::new(),
                    annotations: None,
                    position
                });
            }
            Event::ArrayStop => {
                let node = self.value_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::ObjectStart => {
                self.key_stack.push(Key::create(""));
                self.value_stack.push(Value::Object {
                    value: IndexMap::new(),
                    annotations: None,
                    position,
                });
            }
            Event::ObjectStop => {
                self.key_stack.pop().unwrap();
                let node = self.value_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::Null => {
                let node = Value::Null { annotations: None, position };
                self.insert_new_node(node);
            }
            Event::Float(value) => {
                let node = Value::Float {
                    value,
                    annotations: None,
                    position,
                };
                self.insert_new_node(node);
            }
            Event::Integer(value) => {
                let node = Value::Integer {
                    value,
                    annotations: None,
                    position,
                };
                self.insert_new_node(node);
            }
            Event::Boolean(value) => {
                let node = Value::Boolean {
                    value,
                    annotations: None,
                    position,
                };
                self.insert_new_node(node);
            }
            Event::String(value) => {
                let node = Value::String {
                    value,
                    annotations: None,
                    position,
                };
                self.insert_new_node(node);
            }
            Event::Annotations(annotations) => {
                self.insert_annotations(Some(annotations));
            }
        }
    }
}
