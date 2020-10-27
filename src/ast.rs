use indexmap::IndexMap;

use crate::lexer::Position;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};
use std::string;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-support", serde(tag = "type", content = "value"))]
pub enum Ast {
    Null(Null),
    Boolean(Boolean),
    Integer(Integer),
    Float(Float),
    String(String),
    Array(Array),
    Object(Object),
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Null {
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Boolean {
    pub value: bool,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Integer {
    pub value: i64,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Float {
    pub value: f64,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct String {
    pub value: string::String,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Array {
    pub value: Vec<Ast>,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Object {
    pub value: Vec<Property>,
    pub annotations: Vec<Annotation>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Property {
    pub key: string::String,
    pub position: Position,
    pub value: Ast,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Annotation {
    pub name: string::String,
    pub position: Position,
    pub value: Option<Value>,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-support", serde(untagged))]
pub enum Value {
    Null,
    Bool(bool),
    Float(f64),
    Integer(i64),
    String(string::String),
    Array(Vec<Value>),
    Object(IndexMap<string::String, Value>),
}

impl Ast {
    pub fn get_annotations(&self) -> &Vec<Annotation> {
        match self {
            Ast::Null(Null { annotations, .. }) => annotations,
            Ast::Boolean(Boolean { annotations, .. }) => annotations,
            Ast::Integer(Integer { annotations, .. }) => annotations,
            Ast::Float(Float { annotations, .. }) => annotations,
            Ast::String(String { annotations, .. }) => annotations,
            Ast::Array(Array { annotations, .. }) => annotations,
            Ast::Object(Object { annotations, .. }) => annotations,
        }
    }
    pub fn get_annotations_mut(&mut self) -> &mut Vec<Annotation> {
        match self {
            Ast::Null(Null { annotations, .. }) => annotations,
            Ast::Boolean(Boolean { annotations, .. }) => annotations,
            Ast::Integer(Integer { annotations, .. }) => annotations,
            Ast::Float(Float { annotations, .. }) => annotations,
            Ast::String(String { annotations, .. }) => annotations,
            Ast::Array(Array { annotations, .. }) => annotations,
            Ast::Object(Object { annotations, .. }) => annotations,
        }
    }
}
