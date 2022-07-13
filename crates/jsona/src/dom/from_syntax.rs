use super::{
    error::Error,
    keys::KeyOrIndex,
    node::{
        Annotations, AnnotationsInner, Array, ArrayInner, BoolInner, Entries, FloatInner,
        IntegerInner, IntegerRepr, Invalid, InvalidInner, Key, KeyInner, Node, NullInner, Object,
        ObjectInner, StrInner, StrRepr,
    },
};

use crate::{
    syntax::{SyntaxElement, SyntaxKind::*},
    util::shared::Shared,
};
use either::Either;

pub fn from_syntax(root: SyntaxElement) -> Node {
    if root.kind() != VALUE {
        return invalid_from_syntax(root, None).into();
    }
    let annotations = annotations_from_syntax(root.clone());
    let errors: Vec<Error> = Default::default();
    match first_none_value_child(&root) {
        None => invalid_from_syntax(root, annotations).into(),
        Some(syntax) => match syntax.kind() {
            NULL => NullInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
            }
            .wrap()
            .into(),
            BOOL => BoolInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                value: Default::default(),
            }
            .wrap()
            .into(),
            INTEGER => IntegerInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                value: Default::default(),
                repr: IntegerRepr::Dec,
            }
            .wrap()
            .into(),
            INTEGER_BIN => IntegerInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                value: Default::default(),
                repr: IntegerRepr::Bin,
            }
            .wrap()
            .into(),
            INTEGER_HEX => IntegerInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                value: Default::default(),
                repr: IntegerRepr::Hex,
            }
            .wrap()
            .into(),
            INTEGER_OCT => IntegerInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                value: Default::default(),
                repr: IntegerRepr::Oct,
            }
            .wrap()
            .into(),
            FLOAT => FloatInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                value: Default::default(),
            }
            .wrap()
            .into(),
            SINGLE_QUOTE => StrInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                repr: StrRepr::Single,
                value: Default::default(),
            }
            .wrap()
            .into(),
            DOUBLE_QUOTE => StrInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                repr: StrRepr::Double,
                value: Default::default(),
            }
            .wrap()
            .into(),
            BACKTICK_QUOTE => StrInner {
                errors: errors.into(),
                syntax: Some(syntax),
                root_syntax: Some(root),
                annotations,
                repr: StrRepr::Backtick,
                value: Default::default(),
            }
            .wrap()
            .into(),
            ARRAY => array_from_syntax(root, syntax, annotations).into(),
            OBJECT => object_from_syntax(root, syntax, annotations).into(),
            _ => invalid_from_syntax(root, annotations).into(),
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
                            is_valid: true,
                            value: Default::default(),
                        }
                        .into();

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

pub(crate) fn key_from_syntax(syntax: SyntaxElement) -> Key {
    assert!(syntax.kind() == KEY);
    if let Some(syntax) =
        first_none_value_child(&syntax).and_then(|v| if v.kind() == IDENT { Some(v) } else { None })
    {
        KeyInner {
            errors: Shared::default(),
            syntax: Some(syntax),
            is_valid: true,
            value: Default::default(),
        }
        .into()
    } else {
        KeyInner {
            errors: Shared::new(Vec::from([Error::UnexpectedSyntax {
                syntax: syntax.clone(),
            }])),
            is_valid: false,
            value: Default::default(),
            syntax: Some(syntax),
        }
        .into()
    }
}

fn array_from_syntax(
    root: SyntaxElement,
    syntax: SyntaxElement,
    annotations: Option<Annotations>,
) -> Array {
    assert!(syntax.kind() == ARRAY);
    ArrayInner {
        errors: Default::default(),
        root_syntax: Some(root),
        syntax: Some(syntax.clone()),
        annotations,
        items: Shared::new(
            syntax
                .as_node()
                .unwrap()
                .children()
                .filter(|v| v.kind() == VALUE)
                .map(|syntax| from_syntax(syntax.into()))
                .collect(),
        ),
    }
    .into()
}

fn object_from_syntax(
    root: SyntaxElement,
    syntax: SyntaxElement,
    annotations: Option<Annotations>,
) -> Object {
    assert!(syntax.kind() == OBJECT);
    let mut errors = Vec::new();
    let mut entries = Entries::default();
    for child in syntax
        .as_node()
        .unwrap()
        .children()
        .filter(|v| v.kind() == ENTRY)
    {
        object_entry_from_syntax(child.into(), &mut entries, &mut errors)
    }
    ObjectInner {
        errors: errors.into(),
        root_syntax: Some(root),
        syntax: Some(syntax),
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

fn annotations_from_syntax(syntax: SyntaxElement) -> Option<Annotations> {
    assert!(syntax.kind() == VALUE);
    let syntax = syntax.into_node().unwrap();

    let mut errors: Vec<Error> = vec![];
    let mut entries = Entries::default();
    match (
        syntax.children().find(|v| v.kind() == ANNOTATIONS),
        syntax
            .children()
            .find(|v| v.kind() == OBJECT || v.kind() == ARRAY)
            .and_then(|v| v.children().find(|v| v.kind() == ANNOTATIONS)),
    ) {
        (None, None) => return None,
        (None, Some(scope_annotations)) => {
            for child in scope_annotations.children() {
                anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
            }
        }
        (Some(entry_annotations), None) => {
            for child in entry_annotations.children() {
                anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
            }
        }
        (Some(entry_annotations), Some(scope_annotations)) => {
            for child in entry_annotations.children() {
                anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
            }
            for child in scope_annotations.children() {
                anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
            }
        }
    };
    Some(
        AnnotationsInner {
            errors: errors.into(),
            entries: entries.into(),
        }
        .into(),
    )
}

fn anno_entry_from_syntax(syntax: SyntaxElement, entries: &mut Entries, errors: &mut Vec<Error>) {
    assert!(syntax.kind() == ANNOTATION_ENTRY);
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
        None => NullInner::default().wrap().into(),
    };
    add_entry(entries, errors, key, value);
}

fn invalid_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Invalid {
    let errors = Vec::from([Error::UnexpectedSyntax {
        syntax: syntax.clone(),
    }]);
    InvalidInner {
        errors: errors.into(),
        root_syntax: Some(syntax.clone()),
        syntax: Some(syntax),
        annotations,
    }
    .into()
}

fn first_none_value_child(syntax: &SyntaxElement) -> Option<SyntaxElement> {
    let node = syntax.as_node()?;
    node.children_with_tokens()
        .find(|v| ![WHITESPACE, NEWLINE, BLOCK_COMMENT, LINE_COMMENT].contains(&v.kind()))
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
