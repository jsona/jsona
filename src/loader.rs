use crate::ast::*;
use crate::lexer::Position;
use crate::parser::{Event, EventReceiver, ParseResult, Parser};

pub struct Loader {
    value_stack: Vec<Ast>,
    key_stack: Vec<Option<(Position, String)>>,
}

impl Loader {
    pub fn load_from_str(input: &str) -> ParseResult<Ast> {
        let mut loader = Loader {
            value_stack: Vec::new(),
            key_stack: Vec::new(),
        };
        let mut parser = Parser::new(input.chars());
        parser.parse(&mut loader)?;
        Ok(loader.value_stack.pop().unwrap())
    }
    fn insert_new_node(&mut self, node: Ast) {
        if self.value_stack.is_empty() {
            self.value_stack.push(node);
        } else {
            let parent = self.value_stack.last_mut().unwrap();
            match *parent {
                Ast::Array(Array { ref mut elements, .. }) => elements.push(node),
                Ast::Object(Object { ref mut properties, .. }) => {
                    let cur_key = self.key_stack.pop().unwrap();
                    let new_key = match cur_key {
                        Some((position, key)) => {
                            properties.push(Property {
                                name: key,
                                position,
                                value: node,
                            });
                            None
                        }
                        None => {
                            if let Ast::String(AstString {
                                value, position, ..
                            }) = node
                            {
                                Some((position, value))
                            } else {
                                unreachable!()
                            }
                        }
                    };
                    self.key_stack.push(new_key);
                }
                _ => unreachable!(),
            }
        }
    }
    fn insert_annotations(&mut self, annos: Vec<Anno>) {
        let parent = self.value_stack.last_mut().unwrap();
        match *parent {
            Ast::Array(Array { ref mut elements, .. }) => {
                if elements.len() > 0 {
                    let last_elem = elements.last_mut().unwrap();
                    append_annos(last_elem.get_annotations_mut(), annos)
                } else {
                    append_annos(parent.get_annotations_mut(), annos);
                }
            }
            Ast::Object(Object { ref mut properties, .. }) => {
                if properties.len() > 0 {
                    let last_prop = properties.last_mut().unwrap();
                    append_annos(last_prop.value.get_annotations_mut(), annos)
                } else {
                    append_annos(parent.get_annotations_mut(), annos);
                }
            }
            _ => append_annos(parent.get_annotations_mut(), annos),
        }
    }
}

impl EventReceiver for Loader {
    fn on_event(&mut self, event: Event, position: Position) {
        match event {
            Event::ArrayStart => {
                self.value_stack.push(Ast::Array(Array {
                    elements: Vec::new(),
                    annotations: Vec::new(),
                    position,
                }));
            }
            Event::ArrayStop => {
                let node = self.value_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::ObjectStart => {
                self.key_stack.push(None);
                self.value_stack.push(Ast::Object(Object {
                    properties: Vec::new(),
                    annotations: Vec::new(),
                    position,
                }));
            }
            Event::ObjectStop => {
                self.key_stack.pop().unwrap();
                let node = self.value_stack.pop().unwrap();
                self.insert_new_node(node);
            }
            Event::Null => {
                let node = Ast::Null(Null {
                    annotations: Vec::new(),
                    position,
                });
                self.insert_new_node(node);
            }
            Event::Float(value) => {
                let node = Ast::Float(Float {
                    value,
                    annotations: Vec::new(),
                    position,
                });
                self.insert_new_node(node);
            }
            Event::Integer(value) => {
                let node = Ast::Integer(Integer {
                    value,
                    annotations: Vec::new(),
                    position,
                });
                self.insert_new_node(node);
            }
            Event::Boolean(value) => {
                let node = Ast::Boolean(Boolean {
                    value,
                    annotations: Vec::new(),
                    position,
                });
                self.insert_new_node(node);
            }
            Event::String(value) => {
                let node = Ast::String(AstString {
                    value,
                    annotations: Vec::new(),
                    position,
                });
                self.insert_new_node(node);
            }
            Event::Annotations(annotations) => {
                self.insert_annotations(annotations);
            }
        }
    }
}

fn append_annos(target: &mut Vec<Anno>, annos: Vec<Anno>) {
    for anno in annos {
        target.push(anno);
    }
}
