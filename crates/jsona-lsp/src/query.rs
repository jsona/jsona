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
}

impl Query {
    pub fn at(root: &Node, offset: TextSize, is_completion: bool) -> Self {
        let syntax = root.node_syntax().cloned().unwrap().into_node().unwrap();
        let before = offset
            .checked_sub(TextSize::from(1))
            .and_then(|offset| Self::position_info_at(&syntax, offset));
        let after = if offset >= syntax.text_range().end() {
            None
        } else {
            Self::position_info_at(&syntax, offset)
        };

        let mut kind = ScopeKind::Unknown;
        let mut node_at_offset = offset;
        let mut key = None;
        let mut value = None;
        let mut add_value = true;
        if let Some(token) = before
            .as_ref()
            .and_then(|t| Query::prev_none_ws_comment(t.syntax.clone()))
        {
            let mut fallback = || {
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
                            add_value = !node
                                .siblings_with_tokens(Direction::Next)
                                .any(|v| v.kind() == SyntaxKind::COLON);
                            if is_completion {
                                if let Some(offset) = node
                                    .parent()
                                    .and_then(|v| v.parent())
                                    .and_then(|v| v.text_range().start().checked_add(1.into()))
                                {
                                    node_at_offset = offset;
                                }
                            }
                            kind = ScopeKind::PropertyKey;
                        }
                        SyntaxKind::SCALAR => {
                            kind = ScopeKind::Value;
                            value = Some(node);
                        }
                        SyntaxKind::OBJECT => kind = ScopeKind::Object,
                        SyntaxKind::ARRAY => kind = ScopeKind::Array,
                        _ => {}
                    };
                }
            };
            match token.kind() {
                SyntaxKind::ANNOTATION_KEY => {
                    let exist_value = token
                        .siblings_with_tokens(Direction::Next)
                        .any(|v| v.kind() == SyntaxKind::ANNOTATION_VALUE);
                    if !exist_value && !token.text_range().contains_inclusive(offset) {
                        // out a tag annotation
                        fallback()
                    } else {
                        add_value = !exist_value;
                        kind = ScopeKind::AnnotationKey;
                        key = Some(token);
                    }
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
                SyntaxKind::BRACE_END | SyntaxKind::BRACKET_END => {}
                _ => fallback(),
            };
        }

        Query {
            offset,
            before,
            after,
            scope: kind,
            node_at_offset,
            add_value,
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

    pub fn space_and_comma(&self) -> (&'static str, &'static str) {
        let mut add_comma = true;
        let mut add_space = false;
        if self.scope != ScopeKind::AnnotationKey {
            if let Some(token) = self.before.as_ref().map(|v| &v.syntax) {
                if let Some(parent) = token.parent_ancestors().find(|v| {
                    matches!(
                        v.kind(),
                        SyntaxKind::ANNOTATION_VALUE | SyntaxKind::OBJECT | SyntaxKind::ARRAY
                    )
                }) {
                    let mut found = false;
                    let mut exist_new_line = false;
                    let mut prev_token = None;
                    let mut next_token = None;
                    for event in parent.preorder_with_tokens() {
                        if let WalkEvent::Enter(ele) = event {
                            if let Some(t) = ele.as_token() {
                                if t.text().contains(|p| p == '\r' || p == '\n') {
                                    exist_new_line = true;
                                }
                                if found && !t.kind().is_ws_or_comment() {
                                    next_token = Some(t.clone());
                                    break;
                                }
                                if !found {
                                    if t.text_range().contains_inclusive(self.offset) {
                                        found = true;
                                    } else {
                                        prev_token = Some(t.clone())
                                    }
                                }
                            }
                        }
                    }

                    if let Some(t) = next_token {
                        if matches!(
                            t.kind(),
                            SyntaxKind::COMMA
                                | SyntaxKind::BRACE_END
                                | SyntaxKind::BRACKET_END
                                | SyntaxKind::PARENTHESES_END
                        ) {
                            add_comma = false;
                        }
                    }
                    if exist_new_line {
                        match self.scope {
                            ScopeKind::Array => {}
                            ScopeKind::Value => {
                                if !(self
                                    .before
                                    .as_ref()
                                    .map(|v| v.syntax.kind().is_ws_or_comment())
                                    .unwrap_or_default()
                                    || prev_token
                                        .map(|v| v.kind().is_ws_or_comment())
                                        .unwrap_or_default())
                                {
                                    add_space = true
                                }
                            }
                            _ => {
                                add_space = true;
                            }
                        }
                    }
                }
            }
        }
        let space = if add_space { " " } else { "" };
        let comma = if add_comma { "," } else { "" };
        (space, comma)
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
        for (key, value) in map.iter() {
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
            for (key, value) in map.iter() {
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
