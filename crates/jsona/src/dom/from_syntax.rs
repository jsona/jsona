use super::{
    error::Error,
    keys::KeyOrIndex,
    node::{
        Annotations, AnnotationsInner, Array, ArrayInner, Bool, BoolInner, Comment, DomNode, Entries, Float,
        FloatInner, Integer, IntegerInner, IntegerRepr, Invalid, InvalidInner, Key, KeyInner, Node,
        Null, NullInner, Object, ObjectInner, Str, StrInner, StrRepr,
    },
};

use crate::{
    syntax::{SyntaxElement, SyntaxKind::*, SyntaxNode},
    util::shared::Shared,
};
use either::Either;

pub fn from_syntax(syntax: SyntaxElement) -> Node {
    if syntax.kind() != VALUE {
        return invalid_from_syntax(syntax, None).into();
    }
    let syntax = syntax.into_node().unwrap();
    let annotations = syntax
        .children_with_tokens()
        .find(|v| v.kind() == ANNOTATIONS)
        .map(annotations_from_syntax);
    match first_none_empty_child(&syntax) {
        None => invalid_from_syntax(syntax.into(), None).into(),
        Some(syntax) => match syntax.kind() {
            NULL => null_from_syntax(syntax, annotations).into(),
            BOOL => bool_from_syntax(syntax, annotations).into(),
            INTEGER | INTEGER_HEX | INTEGER_OCT | INTEGER_BIN => {
                integer_from_syntax(syntax, annotations).into()
            }
            FLOAT => float_from_syntax(syntax, annotations).into(),
            SINGLE_QUOTE | DOUBLE_QUOTE | BACKTICK_QUOTE => str_from_syntax(syntax, annotations).into(),
            ARRAY => array_from_syntax(syntax, annotations).into(),
            OBJECT => object_from_syntax(syntax, annotations).into(),
            _ => invalid_from_syntax(syntax, None).into(),
        },
    }
}

pub(crate) fn keys_from_syntax(
    syntax: &SyntaxElement,
) -> impl ExactSizeIterator<Item = KeyOrIndex> {
    assert!(syntax.kind() == KEY);

    syntax
        .as_node()
        .map(|syntax| {
            let mut keys = vec![];
            let mut at_token = false;
            for child in syntax.children_with_tokens() {
                match child.kind() {
                    AT => at_token = true,
                    PERIOD => at_token = false,
                    IDENT => {
                        let key = KeyInner {
                            errors: Shared::default(),
                            syntax: Some(child),
                            annotations: None,
                            is_valid: true,
                            value: Default::default(),
                        }
                        .wrap();

                        if at_token {
                            keys.push(KeyOrIndex::new_anno_key(key));
                        } else {
                            keys.push(KeyOrIndex::new_key(key));
                        }
                    }
                    _ => {}
                }
            }
            Either::Left(keys.into_iter())
        })
        .unwrap_or_else(|| Either::Right(core::iter::empty()))
}

pub(crate) fn comment_from_syntax(syntax: SyntaxElement) -> Comment {
    Comment {
        syntax: Some(syntax),
        value: Default::default(),
    }
}

fn null_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Null {
    assert!(syntax.kind() == NULL);
    NullInner {
        errors: Default::default(),
        syntax: Some(syntax),
        annotations,
        is_omitted: false,
    }
    .into()
}

fn bool_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Bool {
    assert!(syntax.kind() == BOOL);
    BoolInner {
        errors: Default::default(),
        syntax: Some(syntax),
        annotations,
        value: Default::default(),
    }
    .into()
}

fn integer_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Integer {
    let mut errors = Vec::new();
    match syntax.kind() {
        INTEGER => IntegerInner {
            errors: errors.into(),
            syntax: Some(syntax),
            annotations,
            value: Default::default(),
            repr: IntegerRepr::Dec,
        }
        .into(),
        INTEGER_BIN => IntegerInner {
            errors: errors.into(),
            syntax: Some(syntax),
            annotations,
            value: Default::default(),
            repr: IntegerRepr::Bin,
        }
        .into(),
        INTEGER_HEX => IntegerInner {
            errors: errors.into(),
            syntax: Some(syntax),
            annotations,
            value: Default::default(),
            repr: IntegerRepr::Hex,
        }
        .into(),
        INTEGER_OCT => IntegerInner {
            errors: errors.into(),
            syntax: Some(syntax),
            annotations,
            value: Default::default(),
            repr: IntegerRepr::Oct,
        }
        .into(),
        _ => {
            errors.push(Error::UnexpectedSyntax {
                syntax: syntax.clone(),
            });
            IntegerInner {
                errors: errors.into(),
                syntax: Some(syntax),
                annotations,
                value: Default::default(),
                repr: IntegerRepr::Dec,
            }
            .into()
        }
    }
}

fn float_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Float {
    assert!(syntax.kind() == FLOAT);
    FloatInner {
        errors: Default::default(),
        syntax: Some(syntax),
        annotations,
        value: Default::default(),
    }
    .into()
}

fn str_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Str {
    let mut errors = Vec::new();
    match syntax.kind() {
        SINGLE_QUOTE => StrInner {
            errors: errors.into(),
            syntax: Some(syntax),
            annotations,
            repr: StrRepr::Single,
            value: Default::default(),
        }
        .into(),
        DOUBLE_QUOTE => StrInner {
            errors: errors.into(),
            syntax: Some(syntax),
            annotations,
            repr: StrRepr::Double,
            value: Default::default(),
        }
        .into(),
        BACKTICK_QUOTE => StrInner {
            errors: errors.into(),
            syntax: Some(syntax),
            annotations,
            repr: StrRepr::Backtick,
            value: Default::default(),
        }
        .into(),
        _ => {
            errors.push(Error::UnexpectedSyntax {
                syntax: syntax.clone(),
            });
            StrInner {
                errors: errors.into(),
                syntax: Some(syntax),
                annotations,
                repr: StrRepr::Double,
                value: Default::default(),
            }
            .into()
        }
    }
}

fn array_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Array {
    assert!(syntax.kind() == ARRAY);
    let syntax = syntax.into_node().unwrap();
    let mut errors = Vec::new();
    if let Some(annotations) = annotations {
        if let Some(syntax) = annotations.syntax() {
            errors.push(Error::UnexpectedSyntax {
                syntax: syntax.clone(),
            });
        }
    };
    let annotations = syntax
        .children_with_tokens()
        .find(|v| v.kind() == ANNOTATIONS)
        .map(annotations_from_syntax);
    ArrayInner {
        errors: errors.into(),
        syntax: Some(syntax.clone().into()),
        annotations,
        items: Shared::new(
            syntax
                .children()
                .filter(|v| v.kind() == VALUE)
                .map(|syntax| from_syntax(syntax.into()))
                .collect(),
        ),
    }
    .into()
}

fn object_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Object {
    assert!(syntax.kind() == OBJECT);
    let syntax = syntax.into_node().unwrap();
    let mut errors = Vec::new();
    if let Some(annotations) = annotations {
        if let Some(syntax) = annotations.syntax() {
            errors.push(Error::UnexpectedSyntax {
                syntax: syntax.clone(),
            });
        }
    };
    let annotations = syntax
        .children_with_tokens()
        .find(|v| v.kind() == ANNOTATIONS)
        .map(annotations_from_syntax);

    let mut entries = Entries::default();
    for child in syntax.children().filter(|v| v.kind() == ENTRY) {
        object_entry_from_syntax(child.into(), &mut entries, &mut errors)
    }
    ObjectInner {
        errors: errors.into(),
        syntax: Some(syntax.into()),
        annotations,
        entries: entries.into(),
    }
    .into()
}

fn object_entry_from_syntax(syntax: SyntaxElement, entries: &mut Entries, errors: &mut Vec<Error>) {
    assert!(syntax.kind() == ENTRY);
    let syntax = syntax.into_node().unwrap();
    let key = match syntax.children().find(|v| v.kind() == KEY) {
        Some(key) => key_from_syntax(key.into()),
        None => {
            errors.push(Error::UnexpectedSyntax {
                syntax: syntax.into(),
            });
            return;
        }
    };
    let value = match syntax.children().find(|v| v.kind() == VALUE) {
        Some(value) => from_syntax(value.into()),
        None => {
            errors.push(Error::UnexpectedSyntax {
                syntax: syntax.into(),
            });
            return;
        }
    };
    add_entry(entries, errors, key, value);
}

fn annotations_from_syntax(syntax: SyntaxElement) -> Annotations {
    assert!(syntax.kind() == ANNOTATIONS);
    let syntax = syntax.into_node().unwrap();
    let mut errors: Vec<Error> = vec![];
    let mut entries = Entries::default();
    for child in syntax.children() {
        anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
    }
    AnnotationsInner {
        errors: errors.into(),
        entries: entries.into(),
        annotations: None,
        syntax: Some(syntax.into()),
    }
    .wrap()
}

fn anno_entry_from_syntax(syntax: SyntaxElement, entries: &mut Entries, errors: &mut Vec<Error>) {
    assert!(syntax.kind() == ENTRY);
    let syntax = syntax.into_node().unwrap();
    let key = match syntax.children().find(|v| v.kind() == KEY) {
        Some(key) => key_from_syntax(key.into()),
        None => {
            errors.push(Error::UnexpectedSyntax {
                syntax: syntax.into(),
            });
            return;
        }
    };
    let value = match syntax.children().find(|v| v.kind() == ANNOTATION_VALUE) {
        Some(anno_value) => match anno_value.children().find(|v| v.kind() == VALUE) {
            Some(value) => from_syntax(value.into()),
            None => {
                errors.push(Error::UnexpectedSyntax {
                    syntax: syntax.into(),
                });
                return;
            }
        },
        None => Null::new(true).into(),
    };
    add_entry(entries, errors, key, value);
}

fn key_from_syntax(syntax: SyntaxElement) -> Key {
    assert!(syntax.kind() == KEY);
    let syntax = syntax.into_node().unwrap();
    if let Some(syntax) =
        first_none_empty_child(&syntax).and_then(|v| if v.kind() == IDENT { Some(v) } else { None })
    {
        KeyInner {
            errors: Shared::default(),
            syntax: Some(syntax),
            annotations: None,
            is_valid: true,
            value: Default::default(),
        }
        .wrap()
    } else {
        KeyInner {
            errors: Shared::new(Vec::from([Error::UnexpectedSyntax {
                syntax: syntax.clone().into(),
            }])),
            annotations: None,
            is_valid: false,
            value: Default::default(),
            syntax: Some(syntax.into()),
        }
        .wrap()
    }
}

fn invalid_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Invalid {
    let errors = Vec::from([Error::UnexpectedSyntax {
        syntax: syntax.clone(),
    }]);
    InvalidInner {
        errors: errors.into(),
        syntax: Some(syntax),
        annotations,
    }
    .into()
}

fn first_none_empty_child(syntax: &SyntaxNode) -> Option<SyntaxElement> {
    syntax
        .children_with_tokens()
        .find(|v| ![WHITESPACE, NEWLINE, COMMENT_BLOCK, COMMENT_LINE].contains(&v.kind()))
}

/// Add an entry and also collect errors on conflicts.
fn add_entry(entries: &mut Entries, errors: &mut Vec<Error>, key: Key, node: Node) {
    if let Some((existing_key, _)) = entries.lookup.get_key_value(&key) {
        errors.push(Error::ConflictingKeys {
            key: key.clone(),
            other: existing_key.clone(),
        })
    }

    entries.add(key, node);
}
