//! Cursor queries of a JSONA document.

use jsona::{
    dom::{node::DomNode, Keys, Node},
    rowan::{TextRange, TextSize},
    syntax::{SyntaxNode, SyntaxToken},
    util::text_range::join_ranges,
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
    /// Query a DOM root with the given cursor offset.
    /// Returns [`None`] if the position is out of range.
    ///
    /// # Panics
    ///
    /// Panics if the DOM was not entirely constructed from a syntax tree (e.g. if a node has no associated syntax element).
    /// Also panics if the given DOM node is not root.
    ///
    /// Also the given offset must be within the tree.
    #[must_use]
    pub fn at(root: &Node, offset: TextSize) -> Self {
        let syntax = root.syntax().cloned().unwrap().into_node().unwrap();

        Query {
            offset,
            before: offset
                .checked_sub(TextSize::from(1))
                .and_then(|offset| Self::position_info_at(root, &syntax, offset)),
            after: if offset >= syntax.text_range().end() {
                None
            } else {
                Self::position_info_at(root, &syntax, offset)
            },
        }
    }

    fn position_info_at(
        root: &Node,
        syntax: &SyntaxNode,
        offset: TextSize,
    ) -> Option<PositionInfo> {
        let syntax = match syntax.token_at_offset(offset) {
            jsona::rowan::TokenAtOffset::None => return None,
            jsona::rowan::TokenAtOffset::Single(s) => s,
            jsona::rowan::TokenAtOffset::Between(_, right) => right,
        };

        Some(PositionInfo {
            syntax,
            dom_node: root
                .flat_iter()
                .filter(|(k, n)| full_range(k, n).contains(offset))
                .max_by_key(|(k, _)| k.len()),
        })
    }
}

impl Query {}

#[derive(Debug, Clone)]
pub struct PositionInfo {
    /// The narrowest syntax element that contains the position.
    pub syntax: SyntaxToken,
    /// The narrowest node that covers the position.
    pub dom_node: Option<(Keys, Node)>,
}

fn full_range(keys: &Keys, node: &Node) -> TextRange {
    let node_text_rnage = node.node_syntax().unwrap().text_range();
    match keys.last_text_range() {
        Some(text_range) => join_ranges([text_range, node_text_rnage]),
        _ => node_text_rnage,
    }
}
