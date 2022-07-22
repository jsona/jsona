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
    /// Current property/annotaionKey key
    pub key: Option<SyntaxToken>,
    /// Current value
    pub value: Option<SyntaxNode>,
    /// Whether add value for property/annotationKey completion
    pub add_value: bool,
    /// Whether insert seperator
    pub add_seperator: bool,
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
        let mut value = None;
        let mut add_seperator = false;
        let mut add_value = false;
        if let Some(token) = before
            .as_ref()
            .and_then(|t| Query::prev_none_ws_comment(t.syntax.clone()))
        {
            match token.kind() {
                SyntaxKind::ANNOATION_KEY => {
                    add_value = !token
                        .siblings_with_tokens(Direction::Next)
                        .any(|v| v.kind() == SyntaxKind::ANNOTATION_VALUE);
                    kind = ScopeKind::AnnotationKey;
                    key = Some(token);
                }
                SyntaxKind::COLON => {
                    node_at_offset = token.text_range().end();
                    kind = ScopeKind::Value;
                    value = token.next_sibling_or_token().and_then(|v| {
                        if v.kind() == SyntaxKind::VALUE {
                            v.as_node()
                                .unwrap()
                                .children()
                                .find(|v| v.kind() == SyntaxKind::SCALAR)
                        } else {
                            None
                        }
                    });
                    add_seperator = match value.as_ref() {
                        Some(v) => !v
                            .siblings_with_tokens(Direction::Next)
                            .any(|v| v.kind() == SyntaxKind::COMMA),
                        None => !token
                            .next_token()
                            .and_then(Query::next_none_ws_comment)
                            .map(|v| v.kind() == SyntaxKind::COMMA)
                            .unwrap_or_default(),
                    };
                }
                SyntaxKind::PARENTHESES_START => {
                    kind = ScopeKind::Value;
                    value = token.next_sibling_or_token().and_then(|v| {
                        if v.kind() == SyntaxKind::ANNOTATION_VALUE {
                            v.as_node()
                                .unwrap()
                                .children()
                                .find(|v| v.kind() == SyntaxKind::VALUE)
                                .and_then(|v| v.children().find(|v| v.kind() == SyntaxKind::SCALAR))
                        } else {
                            None
                        }
                    });
                }
                SyntaxKind::BRACE_START => {
                    kind = ScopeKind::Object;
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
                        match &node.kind() {
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
                            SyntaxKind::SCALAR => {
                                kind = ScopeKind::Value;
                                add_seperator = !node
                                    .siblings_with_tokens(Direction::Next)
                                    .any(|v| v.kind() == SyntaxKind::COMMA);
                                value = Some(node);
                            }
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
            add_seperator,
            key,
            value,
        }
    }

    pub fn node_at(root: &Node, offset: TextSize, include_key: bool) -> Option<(Keys, Node)> {
        if !is_value_contained(root, offset, None, include_key) {
            return None;
        }
        node_at_impl(root, offset, Keys::default(), include_key)
    }

    pub fn index_at(&self) -> Option<usize> {
        self.before
            .as_ref()
            .and_then(|v| {
                v.syntax
                    .parent_ancestors()
                    .find(|v| v.kind() == SyntaxKind::ARRAY)
            })
            .map(|v| {
                let mut index = 0;
                for child in v.children() {
                    if child.kind() == SyntaxKind::VALUE {
                        index += 1;
                        if child.text_range().contains(self.offset) {
                            break;
                        }
                    }
                }
                index
            })
    }

    fn position_info_at(syntax: &SyntaxNode, offset: TextSize) -> Option<PositionInfo> {
        let syntax = match syntax.token_at_offset(offset) {
            TokenAtOffset::None => return None,
            TokenAtOffset::Single(s) => s,
            TokenAtOffset::Between(_, right) => right,
        };

        Some(PositionInfo { syntax })
    }

    fn prev_none_ws_comment(token: SyntaxToken) -> Option<SyntaxToken> {
        if token.kind().is_ws_or_comment() {
            token.prev_token().and_then(Query::prev_none_ws_comment)
        } else {
            Some(token)
        }
    }

    fn next_none_ws_comment(token: SyntaxToken) -> Option<SyntaxToken> {
        if token.kind().is_ws_or_comment() {
            token.next_token().and_then(Query::next_none_ws_comment)
        } else {
            Some(token)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Unknown,
    Object,
    Array,
    AnnotationKey,
    PropertyKey,
    Value,
}

#[derive(Debug, Clone)]
pub struct PositionInfo {
    /// The narrowest syntax element that contains the position.
    pub syntax: SyntaxToken,
}

fn node_at_impl(
    node: &Node,
    offset: TextSize,
    keys: Keys,
    include_key: bool,
) -> Option<(Keys, Node)> {
    if let Some(annotations) = node.annotations() {
        for (key, value) in annotations.value().read().iter() {
            if is_value_contained(value, offset, Some(key), include_key) {
                return node_at_impl(value, offset, keys.join(key.into()), include_key);
            }
        }
    }
    match node {
        Node::Array(arr) => {
            for (index, value) in arr.value().read().iter().enumerate() {
                if is_value_contained(value, offset, None, include_key) {
                    return node_at_impl(value, offset, keys.join(index.into()), include_key);
                }
            }
        }
        Node::Object(obj) => {
            for (key, value) in obj.value().read().iter() {
                if is_value_contained(value, offset, Some(key), include_key) {
                    return node_at_impl(value, offset, keys.join(key.into()), include_key);
                }
            }
        }
        _ => {}
    }
    Some((keys, node.clone()))
}

fn is_value_contained(
    value: &Node,
    offset: TextSize,
    key: Option<&Key>,
    include_key: bool,
) -> bool {
    match (
        key.and_then(|k| k.node_syntax().map(|v| v.text_range())),
        value.node_text_range(),
    ) {
        (None, Some(range)) => range.contains(offset),
        (Some(range1), Some(range2)) => {
            if include_key {
                range1.cover(range2).contains(offset)
            } else {
                TextRange::empty(range1.end().checked_add(TextSize::from(1)).unwrap())
                    .cover(range2)
                    .contains(offset)
            }
        }
        (Some(range1), None) if include_key => range1.contains(offset),
        _ => false,
    }
}
