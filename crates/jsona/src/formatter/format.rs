use std::rc::Rc;

use super::common::Context;
use super::{Options, OptionsIncomplete};

use crate::{
    dom::{self, DomNode, Keys, Node},
    parser,
    syntax::{SyntaxElement, SyntaxKind::*, SyntaxNode, SyntaxToken},
};
use rowan::{NodeOrToken, TextRange};

/// Parses then formats a JSONA document, skipping ranges that contain syntax errors.
pub fn format(src: &str, options: Options) -> String {
    let p = parser::parse(src);

    let ctx = Context {
        errors: p.errors.iter().map(|err| err.range).collect(),
        ..Context::default()
    };

    format_impl(p.into_syntax(), options, ctx)
}

fn format_impl(node: SyntaxNode, options: Options, context: Context) -> String {
    assert!(node.kind() == VALUE);
    let mut formatted = format_value(node, &options, &context);

    if formatted.ends_with("\r\n") {
        formatted.truncate(formatted.len() - 2);
    } else if formatted.ends_with('\n') {
        formatted.truncate(formatted.len() - 1);
    }

    if options.trailing_newline {
        formatted += options.newline();
    }

    formatted
}

fn format_value(node: SyntaxNode, options: &Options, context: &Context) -> String {
    assert!(node.kind() == VALUE);
    todo!()
}
