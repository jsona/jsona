use jsona_util::environment::Environment;
use lsp_async_stub::{
    rpc::Error,
    util::{LspExt, Position},
    Context, Params,
};
use lsp_types::{SelectionRange, SelectionRangeParams};

use crate::{query::Query, world::World};
#[tracing::instrument(skip_all)]
pub(crate) async fn selection_ranges<E: Environment>(
    context: Context<World<E>>,
    params: Params<SelectionRangeParams>,
) -> Result<Option<Vec<SelectionRange>>, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let document_uri = &p.text_document.uri;
    let ws = workspaces.by_document(document_uri);
    let doc = match ws.document(document_uri) {
        Ok(d) => d,
        Err(error) => {
            tracing::debug!(%error, "failed to get document from workspace");
            return Ok(None);
        }
    };

    Ok(Some(
        p.positions
            .into_iter()
            .flat_map(|position| {
                let offset = match doc.mapper.offset(Position::from_lsp(position)) {
                    Some(ofs) => ofs,
                    None => {
                        tracing::error!(?position, "document position not found");
                        return None;
                    }
                };
                let (_, node) = Query::node_at(&doc.dom, offset)?;
                let range = node.text_range().and_then(|v| doc.mapper.range(v))?;
                Some(SelectionRange {
                    range: range.into_lsp(),
                    parent: None,
                })
            })
            .collect(),
    ))
}
