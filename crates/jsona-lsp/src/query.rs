//! Cursor queries of a JSONA document.

use jsona::{
    dom::{node::DomNode, Keys, Node},
    rowan::{self, TextSize},
    syntax::{SyntaxNode, SyntaxToken},
};

#[derive(Debug, Default)]
pub struct Query {
    /// The offset the query was made for.
    pub offset: TextSize,
    /// Before the cursor.
    pub before: Option<PositionInfo>,
    /// After the cursor.
    pub after: Option<PositionInfo>,
}

impl Query {
    /// Query information about a cursor position in a syntax tree.
    #[must_use]
    pub fn at(root: &Node, offset: TextSize) -> Self {
        let syntax = root.syntax().cloned().unwrap().into_node().unwrap();

        Query {
            offset,
            before: offset
                .checked_sub(TextSize::from(1))
                .and_then(|offset| Self::position_info_at(&syntax, offset)),
            after: if offset >= syntax.text_range().end() {
                None
            } else {
                Self::position_info_at(&syntax, offset)
            },
        }
    }

    #[must_use]
    pub fn dom_at(root: &Node, offset: TextSize) -> Option<(Keys, Node)> {
        if !node_contains_offset(root, offset) {
            return None;
        }
        dom_at_impl(root, offset, Keys::default())
    }

    fn position_info_at(syntax: &SyntaxNode, offset: TextSize) -> Option<PositionInfo> {
        let syntax = match syntax.token_at_offset(offset) {
            rowan::TokenAtOffset::None => return None,
            rowan::TokenAtOffset::Single(s) => s,
            rowan::TokenAtOffset::Between(_, right) => right,
        };

        Some(PositionInfo { syntax })
    }
}

#[derive(Debug, Clone)]
pub struct PositionInfo {
    /// The narrowest syntax element that contains the position.
    pub syntax: SyntaxToken,
}

fn dom_at_impl(node: &Node, offset: TextSize, keys: Keys) -> Option<(Keys, Node)> {
    if let Some(annotations) = node.annotations() {
        for (key, value) in annotations.value().read().iter() {
            if node_contains_offset(key, offset) {
                return None;
            }
            if node_contains_offset(value, offset) {
                return dom_at_impl(value, offset, keys.join(key.into()));
            }
        }
    }
    match node {
        Node::Array(arr) => {
            for (index, value) in arr.value().read().iter().enumerate() {
                if node_contains_offset(value, offset) {
                    return dom_at_impl(value, offset, keys.join(index.into()));
                }
            }
        }
        Node::Object(obj) => {
            for (key, value) in obj.value().read().iter() {
                if node_contains_offset(value, offset) {
                    return dom_at_impl(value, offset, keys.join(key.into()));
                }
            }
        }
        _ => {}
    }
    Some((keys, node.clone()))
}

fn node_contains_offset<T: DomNode>(node: &T, offset: TextSize) -> bool {
    node.node_syntax()
        .map(|v| v.text_range().contains_inclusive(offset))
        .unwrap_or_default()
}
