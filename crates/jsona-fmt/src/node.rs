use std::iter::FromIterator;
use std::rc::Rc;

use crate::{Context, Options, OptionsIncomplete, ScopedOptions};

use jsona::{
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

/// Formats a DOM root node with given scopes.
///
/// **This doesn't check errors of the DOM.**
pub fn format_with_path_scopes<I, S>(
    dom: Node,
    options: Options,
    errors: &[TextRange],
    scopes: I,
) -> Result<String, dom::Error>
where
    I: IntoIterator<Item = (S, OptionsIncomplete)>,
    S: AsRef<str>,
{
    let mut c = Context {
        errors: errors.into(),
        ..Context::default()
    };

    let mut s = Vec::new();

    for (scope, opts) in scopes {
        let keys: Keys = scope.as_ref().parse()?;
        let matched = dom.find_all_matches(keys, false)?;

        for (_, node) in matched {
            s.extend(node.text_ranges().into_iter().map(|r| (r, opts.clone())));
        }
    }

    c.scopes = Rc::new(ScopedOptions::from_iter(s));

    let mut s = format_impl(
        dom.syntax().unwrap().clone().into_node().unwrap(),
        options.clone(),
        c,
    );

    s = s.trim_end().into();

    if options.trailing_newline {
        s += options.newline();
    }

    Ok(s)
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
