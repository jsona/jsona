//! Cursor queries of a JSONA document.

use jsona::{
    dom::{node::DomNode, Key, Keys, Node},
    rowan::{Direction, TextRange, TextSize, TokenAtOffset},
    syntax::{SyntaxKind, SyntaxNode, SyntaxToken},
};

#[derive(Debug)]
pub struct Query {
    /// The offset the query was made for.
    pub offset: TextSize,
    /// Before the cursor.
    pub before: Option<PositionInfo>,
    /// After the cursor.
    pub after: Option<PositionInfo>,
    /// Scope kind
    pub scope: ScopeKind,
    /// Query node contains offset
    pub node_at_offset: TextSize,
    /// Property or annotaion key
    pub key: Option<SyntaxToken>,
    /// Whether add value in suggestion
    pub add_value: bool,
}

impl Query {
    pub fn at(root: &Node, offset: TextSize) -> Self {
        let syntax = root.node_syntax().cloned().unwrap().into_node().unwrap();
        let before = offset
            .checked_sub(TextSize::from(1))
            .and_then(|offset| Self::position_info_at(&syntax, offset));

        let mut kind = ScopeKind::Unknown;
        let mut node_at_offset = offset;
        let mut key = None;
        let mut add_value = false;
        if let Some(token) = before.as_ref().and_then(|v| {
            if v.syntax.kind().is_ws_or_comment() {
                v.syntax.prev_token()
            } else {
                Some(v.syntax.clone())
            }
        }) {
            match token.kind() {
                SyntaxKind::ANNOATION_KEY => {
                    add_value = !token
                        .siblings_with_tokens(Direction::Next)
                        .any(|v| v.kind() == SyntaxKind::ANNOTATION_VALUE);
                    kind = ScopeKind::AnnotationKey;
                    key = Some(token);
                }
                SyntaxKind::PARENTHESES_START | SyntaxKind::BRACE_START => {
                    kind = ScopeKind::Object;
                }
                SyntaxKind::COLON => {
                    kind = ScopeKind::PropertyValue;
                }
                SyntaxKind::BRACKET_START => {
                    kind = ScopeKind::Array;
                }
                _ => {
                    if let Some(node) = token.parent_ancestors().find(|v| {
                        matches!(
                            v.kind(),
                            SyntaxKind::KEY
                                | SyntaxKind::SCALAR
                                | SyntaxKind::OBJECT
                                | SyntaxKind::ARRAY
                        )
                    }) {
                        node_at_offset = node.text_range().start();
                        match node.kind() {
                            SyntaxKind::KEY => {
                                key = node
                                    .children_with_tokens()
                                    .find(|v| v.kind().is_key())
                                    .and_then(|v| v.as_token().cloned());
                                add_value = !token
                                    .siblings_with_tokens(Direction::Next)
                                    .any(|v| v.kind() == SyntaxKind::COLON);
                                kind = ScopeKind::PropertyKey
                            }
                            SyntaxKind::SCALAR => kind = ScopeKind::Value,
                            SyntaxKind::OBJECT => kind = ScopeKind::Object,
                            SyntaxKind::ARRAY => kind = ScopeKind::Array,
                            _ => {}
                        };
                    }
                }
            };
        }

        Query {
            offset,
            before,
            after: if offset >= syntax.text_range().end() {
                None
            } else {
                Self::position_info_at(&syntax, offset)
            },
            scope: kind,
            node_at_offset,
            add_value,
            key,
        }
    }

    #[must_use]
    pub fn node_at(&self, root: &Node) -> Option<(Keys, Node)> {
        if !is_value_contained(root, self.node_at_offset, None) {
            return None;
        }
        node_at_impl(root, self.node_at_offset, Keys::default())
    }

    fn position_info_at(syntax: &SyntaxNode, offset: TextSize) -> Option<PositionInfo> {
        let syntax = match syntax.token_at_offset(offset) {
            TokenAtOffset::None => return None,
            TokenAtOffset::Single(s) => s,
            TokenAtOffset::Between(_, right) => right,
        };

        Some(PositionInfo { syntax })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Unknown,
    Object,
    Array,
    AnnotationKey,
    PropertyKey,
    PropertyValue,
    Value,
}

#[derive(Debug, Clone)]
pub struct PositionInfo {
    /// The narrowest syntax element that contains the position.
    pub syntax: SyntaxToken,
}

fn node_at_impl(node: &Node, offset: TextSize, keys: Keys) -> Option<(Keys, Node)> {
    if let Some(annotations) = node.annotations() {
        for (key, value) in annotations.value().read().iter() {
            if is_value_contained(value, offset, Some(key)) {
                return node_at_impl(value, offset, keys.join(key.into()));
            }
        }
    }
    match node {
        Node::Array(arr) => {
            for (index, value) in arr.value().read().iter().enumerate() {
                if is_value_contained(value, offset, None) {
                    return node_at_impl(value, offset, keys.join(index.into()));
                }
            }
        }
        Node::Object(obj) => {
            for (key, value) in obj.value().read().iter() {
                if is_value_contained(value, offset, Some(key)) {
                    return node_at_impl(value, offset, keys.join(key.into()));
                }
            }
        }
        _ => {}
    }
    Some((keys, node.clone()))
}

fn is_value_contained(value: &Node, offset: TextSize, key: Option<&Key>) -> bool {
    match (
        key.and_then(|k| k.node_syntax().map(|v| v.text_range())),
        value.node_text_range(),
    ) {
        (None, Some(range)) => range.contains(offset),
        (Some(range1), Some(range2)) => {
            TextRange::empty(range1.end().checked_add(TextSize::from(1)).unwrap())
                .cover(range2)
                .contains(offset)
        }
        _ => false,
    }
}
