use jsona::formatter;
use jsona_util::environment::Environment;
use lsp_async_stub::{rpc::Error, util::LspExt, Context, Params};
use lsp_types::{DocumentFormattingParams, TextEdit};

use crate::World;

#[tracing::instrument(skip_all)]
pub(crate) async fn format<E: Environment>(
    context: Context<World<E>>,
    params: Params<DocumentFormattingParams>,
) -> Result<Option<Vec<TextEdit>>, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let document_uri = &p.text_document.uri;
    let (ws, doc) = workspaces.try_get_document(document_uri)?;

    let mut format_opts = formatter::Options {
        indent_string: if p.options.insert_spaces {
            " ".repeat(p.options.tab_size as usize)
        } else {
            "\t".into()
        },
        ..Default::default()
    };

    if let Some(v) = p.options.insert_final_newline {
        format_opts.trailing_newline = v;
    }

    format_opts.update_camel(ws.lsp_config.formatter.clone());

    Ok(Some(vec![TextEdit {
        range: doc.mapper.all_range().into_lsp(),
        new_text: jsona::formatter::format_syntax(doc.parse.clone().into_syntax(), format_opts),
    }]))
}
