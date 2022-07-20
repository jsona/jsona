use jsona_util::environment::Environment;
use lsp_async_stub::{
    rpc::Error,
    util::{LspExt, Position},
    Context, Params,
};
use lsp_types::{CompletionParams, CompletionResponse};

use crate::query::Query;
use crate::World;

#[tracing::instrument(skip_all)]
pub async fn completion<E: Environment>(
    context: Context<World<E>>,
    params: Params<CompletionParams>,
) -> Result<Option<CompletionResponse>, Error> {
    let p = params.required()?;

    let document_uri = p.text_document_position.text_document.uri;

    let workspaces = context.workspaces.read().await;
    let ws = workspaces.by_document(&document_uri);

    // All completions are tied to schemas.
    if !ws.config.schema.enabled {
        return Ok(None);
    }

    let doc = ws.document(&document_uri)?;

    // let schema_association = match ws.schemas.associations().association_for(&document_uri) {
    //     Some(ass) => ass,
    //     None => return Ok(None),
    // };

    let position = p.text_document_position.position;
    let offset = match doc.mapper.offset(Position::from_lsp(position)) {
        Some(ofs) => ofs,
        None => {
            tracing::error!(?position, "document position not found");
            return Ok(None);
        }
    };

    let query = Query::at(&doc.dom, offset);
    let node_info = Query::dom_at(&doc.dom, offset);
    let node_info = node_info.map(|(k, v)| (k.to_string(), v.to_plain_json()));

    tracing::info!(?query, ?node_info, "debug completion");

    Ok(None)
}
