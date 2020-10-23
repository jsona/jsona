use crate::lexer::Position;
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-support", serde(tag="type", content="value"))]
pub enum Ast {
    Null(Null),
    Boolean(Boolean),
    Integer(Integer),
    Float(Float),
    String(AstString),
    Array(Array),
    Object(Object),
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Null {
    pub annotations: Vec<Anno>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Boolean {
    pub value: bool,
    pub annotations: Vec<Anno>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Integer {
    pub value: i64,
    pub annotations: Vec<Anno>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Float {
    pub value: f64,
    pub annotations: Vec<Anno>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct AstString {
    pub value: String,
    pub annotations: Vec<Anno>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Array {
    pub elements: Vec<Ast>,
    pub annotations: Vec<Anno>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Object {
    pub properties: Vec<Property>,
    pub annotations: Vec<Anno>,
    pub position: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Property {
    pub name: String,
    pub position: Position,
    pub value: Ast,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Anno {
    pub name: String,
    pub position: Position,
    pub fields: Vec<AnnoField>,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct AnnoField {
    pub key: AnnoFieldKey,
    pub value: AnnoFieldValue,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct AnnoFieldKey {
    pub value: String,
    pub position: Position,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-support", serde(tag="type", content="value"))]
pub enum AnnoFieldValue {
    Null,
    Bool(bool),
    Float(f64),
    Integer(i64),
    String(String),
}

impl Ast {
    pub fn get_annotations_mut(&mut self) -> &mut Vec<Anno> {
        match self {
            Ast::Null(Null { annotations, .. }) => annotations,
            Ast::Boolean(Boolean { annotations, .. }) => annotations,
            Ast::Integer(Integer { annotations, .. }) => annotations,
            Ast::Float(Float { annotations, .. }) => annotations,
            Ast::String(AstString { annotations, .. }) => annotations,
            Ast::Array(Array { annotations, .. }) => annotations,
            Ast::Object(Object { annotations, .. }) => annotations,
        }
    }
}
