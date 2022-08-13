use crate::{
    query::{Query, ScopeKind},
    world::DocumentState,
};
use itertools::Itertools;
use jsona::rowan::TextRange;
use jsona_util::environment::Environment;
use lsp_async_stub::{
    rpc::Error,
    util::{LspExt, Position},
    Context, Params,
};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};

use crate::World;

#[tracing::instrument(skip_all)]
pub(crate) async fn hover<E: Environment>(
    context: Context<World<E>>,
    params: Params<HoverParams>,
) -> Result<Option<Hover>, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let document_uri = &p.text_document_position_params.text_document.uri;
    let ws = workspaces.by_document(document_uri);

    if !ws.config.schema.enabled {
        return Ok(None);
    }

    let doc = match ws.document(document_uri) {
        Ok(d) => d,
        Err(error) => {
            tracing::debug!(%error, "failed to get document from workspace");
            return Ok(None);
        }
    };

    let position = p.text_document_position_params.position;
    let offset = match doc.mapper.offset(Position::from_lsp(position)) {
        Some(ofs) => ofs,
        None => {
            tracing::error!(?position, "document position not found");
            return Ok(None);
        }
    };

    let query = Query::at(&doc.dom, offset);
    if query.scope == ScopeKind::Unknown || (query.key.is_none() && query.value.is_none()) {
        return Ok(None);
    }

    let (keys, _) = match Query::node_at(&doc.dom, offset) {
        Some(v) => v,
        None => return Ok(None),
    };

    let schemas = match ws.schemas_at_path(document_uri, &keys).await {
        Some(v) => v,
        None => return Ok(None),
    };

    if let Some(key) = query.key.as_ref() {
        let content = schemas
            .iter()
            .flat_map(|schema| schema.description.clone())
            .join("\n\n");
        return Ok(Some(create_hover(doc, content, key.text_range())));
    } else if let Some(node) = query.value.as_ref() {
        let content = schemas
            .iter()
            .flat_map(|schema| schema.description.clone())
            .join("\n\n");
        return Ok(Some(create_hover(doc, content, node.text_range())));
    }

    Ok(None)
}

fn create_hover(doc: &DocumentState, content: String, text_range: TextRange) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: Some(doc.mapper.range(text_range).unwrap().into_lsp()),
    }
}
