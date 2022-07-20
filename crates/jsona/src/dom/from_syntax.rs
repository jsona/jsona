use rowan::NodeOrToken;

use super::{
    error::Error,
    keys::KeyOrIndex,
    node::{
        Annotations, AnnotationsInner, ArrayInner, BoolInner, Entries, Key, KeyInner, Node,
        NullInner, NumberInner, NumberRepr, ObjectInner, StrRepr, StringInner,
    },
};
use serde_json::Number as JsonNumber;

use crate::{
    syntax::{SyntaxElement, SyntaxKind::*},
    util::shared::Shared,
};

pub fn from_syntax(root: SyntaxElement) -> Node {
    if root.kind() != VALUE {
        return invalid_from_syntax(root, None);
    }
    let annotations = annotations_from_syntax(root.clone());
    match first_value_child(&root) {
        None => invalid_from_syntax(root, annotations),
        Some(syntax) => match syntax.kind() {
            SCALAR => scalar_from_syntax(root, syntax, annotations),
            ARRAY => array_from_syntax(root, syntax, annotations),
            OBJECT => object_from_syntax(root, syntax, annotations),
            _ => invalid_from_syntax(root, annotations),
        },
    }
}

pub(crate) fn keys_from_syntax(
    syntax: &SyntaxElement,
) -> impl ExactSizeIterator<Item = KeyOrIndex> {
    assert!(syntax.kind() == KEYS);

    syntax
        .as_node()
        .map(|syntax| {
            let mut keys = vec![];
            let mut after_bracket = false;
            for child in syntax.children_with_tokens() {
                let child = match child {
                    NodeOrToken::Node(_) => continue,
                    NodeOrToken::Token(v) => v,
                };
                match child.kind() {
                    BRACKET_START => after_bracket = true,
                    ANNOATION_KEY => {
                        let key = KeyInner {
                            errors: Shared::default(),
                            syntax: Some(child.clone().into()),
                            value: child.to_string().into(),
                            is_annotation: true,
                        }
                        .into();
                        keys.push(KeyOrIndex::AnnotationKey(key));
                    }
                    k if k.is_key() => {
                        let text = child.text();
                        let key = KeyInner {
                            errors: Shared::default(),
                            syntax: Some(child.clone().into()),
                            value: Default::default(),
                            is_annotation: false,
                        }
                        .into();
                        if after_bracket {
                            if k == INTEGER {
                                if let Ok(idx) = text.parse::<usize>() {
                                    keys.push(KeyOrIndex::Index(idx));
                                }
                            } else if k == IDENT_WITH_GLOB {
                                keys.push(KeyOrIndex::GlobIndex(text.to_string()));
                            } else {
                                keys.push(KeyOrIndex::PropertyKey(key))
                            }
                        } else if k == IDENT_WITH_GLOB {
                            if text == "**" {
                                match keys.last() {
                                    Some(KeyOrIndex::AnyRecursive) => {}
                                    _ => {
                                        keys.push(KeyOrIndex::AnyRecursive);
                                    }
                                }
                            } else {
                                keys.push(KeyOrIndex::GlobKey(text.to_string()));
                            }
                        } else {
                            keys.push(KeyOrIndex::PropertyKey(key))
                        }
                        after_bracket = false;
                    }
                    _ => {}
                }
            }
            keys.into_iter()
        })
        .unwrap_or_else(|| vec![].into_iter())
}

pub(crate) fn key_from_syntax(syntax: SyntaxElement) -> Key {
    assert!(syntax.kind() == KEY);
    if let Some(child) =
        first_value_child(&syntax).and_then(|v| if v.kind().is_key() { Some(v) } else { None })
    {
        KeyInner {
            errors: Shared::default(),
            syntax: Some(child),
            value: Default::default(),
            is_annotation: false,
        }
        .into()
    } else {
        KeyInner {
            errors: Shared::new(Vec::from([Error::UnexpectedSyntax {
                syntax: syntax.clone(),
            }])),
            syntax: Some(syntax),
            value: Default::default(),
            is_annotation: false,
        }
        .into()
    }
}

fn scalar_from_syntax(
    root: SyntaxElement,
    syntax: SyntaxElement,
    annotations: Option<Annotations>,
) -> Node {
    assert!(syntax.kind() == SCALAR);
    let errors: Vec<Error> = Default::default();
    let syntax = match syntax.into_node().and_then(|v| v.first_child_or_token()) {
        Some(v) => v,
        _ => return invalid_from_syntax(root, annotations),
    };
    match syntax.kind() {
        NULL => NullInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
        }
        .wrap()
        .into(),
        BOOL => BoolInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            value: Default::default(),
        }
        .wrap()
        .into(),
        INTEGER => NumberInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            value: Default::default(),
            repr: NumberRepr::Dec,
        }
        .wrap()
        .into(),
        INTEGER_BIN => NumberInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            value: Default::default(),
            repr: NumberRepr::Bin,
        }
        .wrap()
        .into(),
        INTEGER_HEX => NumberInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            value: Default::default(),
            repr: NumberRepr::Hex,
        }
        .wrap()
        .into(),
        INTEGER_OCT => NumberInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            value: Default::default(),
            repr: NumberRepr::Oct,
        }
        .wrap()
        .into(),
        FLOAT => {
            if let Some(v) = syntax
                .to_string()
                .parse::<f64>()
                .ok()
                .and_then(JsonNumber::from_f64)
            {
                NumberInner {
                    errors: errors.into(),
                    syntax: Some(syntax),
                    node_syntax: Some(root),
                    annotations,
                    value: v.into(),
                    repr: NumberRepr::Float,
                }
                .wrap()
                .into()
            } else {
                invalid_from_syntax(root, annotations)
            }
        }
        SINGLE_QUOTE => StringInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            repr: StrRepr::Single,
            value: Default::default(),
        }
        .wrap()
        .into(),
        DOUBLE_QUOTE => StringInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            repr: StrRepr::Double,
            value: Default::default(),
        }
        .wrap()
        .into(),
        BACKTICK_QUOTE => StringInner {
            errors: errors.into(),
            syntax: Some(syntax),
            node_syntax: Some(root),
            annotations,
            repr: StrRepr::Backtick,
            value: Default::default(),
        }
        .wrap()
        .into(),
        _ => invalid_from_syntax(root, annotations),
    }
}

fn array_from_syntax(
    root: SyntaxElement,
    syntax: SyntaxElement,
    annotations: Option<Annotations>,
) -> Node {
    assert!(syntax.kind() == ARRAY);
    let syntax = match syntax.into_node() {
        Some(v) => v,
        _ => return invalid_from_syntax(root, annotations),
    };
    let items: Vec<Node> = syntax
        .children()
        .filter(|v| v.kind() == VALUE)
        .map(|syntax| from_syntax(syntax.into()))
        .collect();

    ArrayInner {
        errors: Default::default(),
        node_syntax: Some(root),
        syntax: Some(syntax.into()),
        annotations,
        items: items.into(),
    }
    .wrap()
    .into()
}

fn object_from_syntax(
    root: SyntaxElement,
    syntax: SyntaxElement,
    annotations: Option<Annotations>,
) -> Node {
    assert!(syntax.kind() == OBJECT);
    let syntax = match syntax.into_node() {
        Some(v) => v,
        _ => return invalid_from_syntax(root, annotations),
    };
    let mut errors = Vec::new();
    let mut entries = Entries::default();
    for child in syntax.children().filter(|v| v.kind() == PROPERTY) {
        object_entry_from_syntax(child.into(), &mut entries, &mut errors)
    }
    ObjectInner {
        errors: errors.into(),
        node_syntax: Some(root),
        syntax: Some(syntax.into()),
        annotations,
        properties: entries.into(),
    }
    .wrap()
    .into()
}

fn object_entry_from_syntax(syntax: SyntaxElement, entries: &mut Entries, errors: &mut Vec<Error>) {
    assert!(syntax.kind() == PROPERTY);
    let syntax = match syntax.into_node() {
        Some(v) => v,
        None => return,
    };
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
    let syntax = syntax.into_node()?;

    let mut errors: Vec<Error> = vec![];
    let mut entries = Entries::default();
    match (
        syntax.children().find(|v| v.kind() == ANNOTATIONS),
        syntax
            .children()
            .find(|v| v.kind().is_compose())
            .and_then(|v| v.children().find(|v| v.kind() == ANNOTATIONS)),
    ) {
        (None, None) => return None,
        (None, Some(inner_annotations)) => {
            for child in inner_annotations.children() {
                anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
            }
        }
        (Some(outer_annotations), None) => {
            for child in outer_annotations.children() {
                anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
            }
        }
        (Some(outer_annotations), Some(inner_annotations)) => {
            for child in inner_annotations.children() {
                anno_entry_from_syntax(child.into(), &mut entries, &mut errors);
            }
            for child in outer_annotations.children() {
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
    assert!(syntax.kind() == ANNOTATION_PROPERTY);
    let syntax = match syntax.into_node() {
        Some(v) => v,
        None => return,
    };
    let key = match syntax
        .children_with_tokens()
        .find(|v| v.kind() == ANNOATION_KEY)
    {
        Some(key) => KeyInner {
            errors: Shared::default(),
            syntax: Some(key.clone()),
            value: key.to_string().into(),
            is_annotation: true,
        }
        .into(),
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

fn invalid_from_syntax(syntax: SyntaxElement, annotations: Option<Annotations>) -> Node {
    let errors = Vec::from([Error::UnexpectedSyntax {
        syntax: syntax.clone(),
    }]);
    NullInner {
        errors: errors.into(),
        node_syntax: Some(syntax.clone()),
        syntax: Some(syntax),
        annotations,
    }
    .wrap()
    .into()
}

fn first_value_child(syntax: &SyntaxElement) -> Option<SyntaxElement> {
    let node = syntax.as_node()?;
    node.children_with_tokens()
        .find(|v| !v.kind().is_ws_or_comment())
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
