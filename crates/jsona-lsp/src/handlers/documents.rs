use jsona_util::{
    environment::Environment,
    schema::associations::{source, AssociationRule},
};
use lsp_async_stub::{util::Mapper, Context, Params, RequestWriter};
use lsp_types::{
    notification, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    PublishDiagnosticsParams,
};
use url::Url;

use crate::{
    diagnostics,
    world::{DocumentState, World},
    NAME,
};

#[tracing::instrument(skip_all)]
pub(crate) async fn document_open<E: Environment>(
    context: Context<World<E>>,
    params: Params<DidOpenTextDocumentParams>,
) {
    let p = match params.optional() {
        None => return,
        Some(p) => p,
    };

    let parse = jsona::parser::parse(&p.text_document.text);
    let mapper = Mapper::new_utf16(&p.text_document.text, false);

    let mut workspaces = context.workspaces.write().await;
    let document_uri = &p.text_document.uri;
    let ws = workspaces.by_document_mut(document_uri);
    let dom = parse.clone().into_dom();

    if ws.lsp_config.schema.enabled {
        ws.schemas
            .associations()
            .retain(|(rule, assoc)| match rule {
                AssociationRule::Url(u) => {
                    !(u == document_uri && assoc.meta["source"] != source::SCHEMA_FIELD)
                }
                _ => true,
            });
        ws.schemas
            .associations()
            .add_from_document(document_uri, &dom);
        ws.emit_association(context.clone(), document_uri).await;
    }

    ws.documents
        .insert(document_uri.clone(), DocumentState { parse, dom, mapper });

    let ws_root = ws.root.clone();
    drop(workspaces);
    diagnostics::publish_diagnostics(context.clone(), ws_root, document_uri.clone()).await;
}

#[tracing::instrument(skip_all)]
pub(crate) async fn document_change<E: Environment>(
    context: Context<World<E>>,
    params: Params<DidChangeTextDocumentParams>,
) {
    let mut p = match params.optional() {
        None => return,
        Some(p) => p,
    };

    // We expect one full change
    let change = match p.content_changes.pop() {
        None => return,
        Some(c) => c,
    };

    let parse = jsona::parser::parse(&change.text);
    let mapper = Mapper::new_utf16(&change.text, false);

    let mut workspaces = context.workspaces.write().await;
    let document_uri = &p.text_document.uri;
    let ws = workspaces.by_document_mut(document_uri);
    let dom = parse.clone().into_dom();

    if ws.lsp_config.schema.enabled {
        ws.schemas
            .associations()
            .retain(|(rule, assoc)| match rule {
                AssociationRule::Url(u) => {
                    !(u == document_uri && assoc.meta["source"] != source::SCHEMA_FIELD)
                }
                _ => true,
            });
        ws.schemas
            .associations()
            .add_from_document(document_uri, &dom);
        ws.emit_association(context.clone(), document_uri).await;
    }

    ws.documents
        .insert(document_uri.clone(), DocumentState { parse, dom, mapper });

    let ws_root = ws.root.clone();
    drop(workspaces);
    diagnostics::publish_diagnostics(context.clone(), ws_root, document_uri.clone()).await;
}

#[tracing::instrument(skip_all)]
pub(crate) async fn document_save<E: Environment>(
    _context: Context<World<E>>,
    _params: Params<DidSaveTextDocumentParams>,
) {
    // stub to silence warnings
}

#[tracing::instrument(skip_all)]
pub(crate) async fn document_close<E: Environment>(
    context: Context<World<E>>,
    params: Params<DidCloseTextDocumentParams>,
) {
    let p = match params.optional() {
        None => return,
        Some(p) => p,
    };

    let mut workspaces = context.workspaces.write().await;
    let document_uri = &p.text_document.uri;
    let ws = workspaces.by_document_mut(document_uri);

    ws.documents.remove(document_uri);
    drop(workspaces);

    context.env.spawn_local(diagnostics::clear_diagnostics(
        context.clone(),
        document_uri.clone(),
    ));
}

#[tracing::instrument(skip_all, fields(%file))]
async fn hint_excluded<E: Environment>(mut context: Context<World<E>>, file: &Url) {
    context
        .write_notification::<notification::PublishDiagnostics, _>(Some(PublishDiagnosticsParams {
            uri: file.clone(),
            diagnostics: vec![Diagnostic {
                range: Default::default(),
                severity: Some(DiagnosticSeverity::HINT),
                code: None,
                code_description: None,
                source: Some(NAME.into()),
                message: "this document has been excluded".into(),
                related_information: None,
                tags: None,
                data: None,
            }],
            version: None,
        }))
        .await
        .unwrap_or_else(|err| tracing::error!("{err}"));
}
