//! Cursor queries of a JSONA document.

use jsona::{
    dom::{node::DomNode, Keys, Node},
    rowan::{Direction, TextSize, TokenAtOffset, WalkEvent},
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
    /// Current property/annotationKey key
    pub key: Option<SyntaxToken>,
    /// Current value
    pub value: Option<SyntaxNode>,
    /// Whether add value for property/annotationKey completion
    pub add_value: bool,
    /// Whether insert space in value completion
    pub add_space: bool,
    /// Is compact node
    pub compact: bool,
    /// Whether insert separator
    pub add_separator: bool,
}

impl Query {
    pub fn at(root: &Node, offset: TextSize) -> Self {
        let syntax = root.node_syntax().cloned().unwrap().into_node().unwrap();
        let before = offset
            .checked_sub(TextSize::from(1))
            .and_then(|offset| Self::position_info_at(&syntax, offset));
        let compact = is_compact_node(&before);

        let mut kind = ScopeKind::Unknown;
        let mut node_at_offset = offset;
        let mut key = None;
        let mut value = None;
        let mut add_separator = false;
        let mut add_value = true;
        let mut add_space = false;
        if let Some(token) = before
            .as_ref()
            .and_then(|t| Query::prev_none_ws_comment(t.syntax.clone()))
        {
            match token.kind() {
                SyntaxKind::ANNOTATION_KEY => {
                    add_value = !token
                        .siblings_with_tokens(Direction::Next)
                        .any(|v| v.kind() == SyntaxKind::ANNOTATION_VALUE);
                    kind = ScopeKind::AnnotationKey;
                    key = Some(token);
                }
                SyntaxKind::COLON => {
                    node_at_offset = token.text_range().start();
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
                    if !is_compact_node(&before)
                        && before
                            .as_ref()
                            .map(|v| v.syntax.kind() == SyntaxKind::COLON)
                            .unwrap_or_default()
                    {
                        add_space = true;
                    }
                    add_separator = match value.as_ref() {
                        Some(v) => value_add_separator(v),
                        None => colon_add_separator(token),
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
                                kind = ScopeKind::PropertyKey;
                            }
                            SyntaxKind::SCALAR => {
                                kind = ScopeKind::Value;
                                add_separator = value_add_separator(&node);
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
            add_separator,
            add_space,
            compact,
            key,
            value,
        }
    }

    pub fn node_at(root: &Node, offset: TextSize) -> Option<(Keys, Node)> {
        if !root
            .node_text_range()
            .map(|v| v.contains(offset))
            .unwrap_or_default()
        {
            return None;
        }
        node_at_impl(root, offset, Keys::default())
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

fn node_at_impl(node: &Node, offset: TextSize, keys: Keys) -> Option<(Keys, Node)> {
    if let Some(annotations) = node.annotations() {
        let map = annotations.value().read();
        for (key, value) in map.kv_iter() {
            if map
                .syntax(key)
                .map(|v| v.text_range().contains(offset))
                .unwrap_or_default()
            {
                return node_at_impl(value, offset, keys.join(key.into()));
            }
        }
    }
    match node {
        Node::Array(arr) => {
            for (index, value) in arr.value().read().iter().enumerate() {
                if value
                    .node_syntax()
                    .map(|v| v.text_range().contains(offset))
                    .unwrap_or_default()
                {
                    return node_at_impl(value, offset, keys.join(index.into()));
                }
            }
        }
        Node::Object(obj) => {
            let map = obj.value().read();
            for (key, value) in map.kv_iter() {
                if map
                    .syntax(key)
                    .map(|v| v.text_range().contains(offset))
                    .unwrap_or_default()
                {
                    if value.syntax().is_none()
                        && key
                            .syntax()
                            .map(|v| v.text_range().contains(offset))
                            .unwrap_or_default()
                    {
                        return Some((keys, node.clone()));
                    }
                    return node_at_impl(value, offset, keys.join(key.into()));
                }
            }
        }
        _ => {}
    }
    Some((keys, node.clone()))
}

fn value_add_separator(node: &SyntaxNode) -> bool {
    for syntax in node.siblings_with_tokens(Direction::Next) {
        match syntax.kind() {
            SyntaxKind::NEWLINE => return true,
            SyntaxKind::COMMA => return false,
            SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT => {}
            _ => return false,
        }
    }
    false
}

fn colon_add_separator(mut token: SyntaxToken) -> bool {
    loop {
        match token.next_token() {
            Some(tok) => match tok.kind() {
                SyntaxKind::NEWLINE => return true,
                SyntaxKind::COMMA => return false,
                SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT => {
                    token = tok;
                    continue;
                }
                _ => return false,
            },
            None => return false,
        }
    }
}

fn is_compact_node(before: &Option<PositionInfo>) -> bool {
    if let Some(token) = before.as_ref().map(|v| &v.syntax) {
        if let Some(parent) = token.parent_ancestors().find(|v| {
            matches!(
                v.kind(),
                SyntaxKind::ANNOTATION_VALUE | SyntaxKind::OBJECT | SyntaxKind::ARRAY
            )
        }) {
            for event in parent.preorder_with_tokens() {
                if let WalkEvent::Enter(ele) = event {
                    if let Some(t) = ele.as_token() {
                        if t.kind() == SyntaxKind::NEWLINE {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}
