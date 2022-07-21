use jsona::dom::{DomNode, Key, Keys};
use jsona_schema::Schema;
use jsona_util::environment::Environment;
use lsp_async_stub::{
    rpc::Error,
    util::{LspExt, Position},
    Context, Params,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, CompletionTextEdit,
    Documentation, MarkupContent, TextEdit,
};

use crate::World;
use crate::{
    query::{Query, ScopeKind},
    world::DocumentState,
};

#[tracing::instrument(skip_all)]
pub async fn completion<E: Environment>(
    context: Context<World<E>>,
    params: Params<CompletionParams>,
) -> Result<Option<CompletionResponse>, Error> {
    let p = params.required()?;

    let document_uri = p.text_document_position.text_document.uri;

    let workspaces = context.workspaces.read().await;
    let ws = workspaces.by_document(&document_uri);

    if !ws.config.schema.enabled {
        return Ok(None);
    }

    let doc = ws.document(&document_uri)?;

    let schema_association = match ws.schemas.associations().association_for(&document_uri) {
        Some(ass) => ass,
        None => return Ok(None),
    };

    let position = p.text_document_position.position;
    let offset = match doc.mapper.offset(Position::from_lsp(position)) {
        Some(ofs) => ofs,
        None => {
            tracing::error!(?position, "document position not found");
            return Ok(None);
        }
    };

    let query = Query::at(&doc.dom, offset);
    if query.scope == ScopeKind::Unknown {
        return Ok(None);
    }

    let (mut keys, node) = match query.node_at(&doc.dom) {
        Some(v) => v,
        None => return Ok(None),
    };

    if query.scope == ScopeKind::AnnotationKey {
        keys = Keys::single(Key::annotation(query.key.as_ref().unwrap().text()))
    }

    let schemas = match ws
        .schemas
        .pointer_schemas(&schema_association.url, &keys)
        .await
    {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(?error, "failed to query schemas");
            return Ok(None);
        }
    };

    tracing::info!(?query, "debug completion keys={}", keys);

    match &query.scope {
        ScopeKind::AnnotationKey => {
            let props = node
                .annotations()
                .map(|v| v.members_keys())
                .unwrap_or_default();
            return Ok(complete_properties(doc, &schemas, &props, &query));
        }
        ScopeKind::PropertyKey | ScopeKind::Object => {
            let props = node
                .as_object()
                .map(|v| v.properties_keys())
                .unwrap_or_default();
            return Ok(complete_properties(doc, &schemas, &props, &query));
        }
        _ => {}
    }

    Ok(None)
}

fn complete_properties(
    doc: &DocumentState,
    schemas: &[Schema],
    exist_props: &[String],
    query: &Query,
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    for schema in schemas.iter() {
        tracing::info!(
            "complete properties schema={}",
            serde_json::to_string(schema).unwrap()
        );
        if !schema.is_object() {
            continue;
        }
        match schema.properties.as_ref() {
            None => continue,
            Some(properties) => {
                for (prop_key, prop_value) in properties {
                    if exist_props.contains(prop_key) {
                        continue;
                    }
                    comp_items.push(CompletionItem {
                        label: prop_key.to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        documentation: make_doc(prop_value),
                        text_edit: query.key.as_ref().map(|r| {
                            CompletionTextEdit::Edit(TextEdit {
                                range: doc.mapper.range(r.text_range()).unwrap().into_lsp(),
                                new_text: prop_key.to_string(),
                            })
                        }),
                        ..Default::default()
                    })
                }
            }
        }
    }
    if comp_items.is_empty() {
        None
    } else {
        Some(CompletionResponse::Array(comp_items))
    }
}

fn make_doc(schema: &Schema) -> Option<Documentation> {
    if let Some(docs) = schema.description.as_ref() {
        return Some(Documentation::MarkupContent(MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: docs.into(),
        }));
    }
    None
}
