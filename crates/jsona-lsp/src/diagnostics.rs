use crate::{
    world::{DocumentState, WorkspaceState, World},
    NAME,
};
use jsona::dom::Node;
use jsona_util::environment::Environment;
use lsp_async_stub::{util::LspExt, Context, RequestWriter};
use lsp_types::{
    notification, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    PublishDiagnosticsParams, Url,
};

#[tracing::instrument(skip_all)]
pub(crate) async fn publish_diagnostics<E: Environment>(
    mut context: Context<World<E>>,
    ws_url: Url,
    document_url: Url,
) {
    let mut diags = Vec::new();

    let workspaces = context.workspaces.read().await;
    let ws = match workspaces.get(&ws_url) {
        Some(d) => d,
        None => {
            tracing::warn!(%document_url, "workspace not found");
            return;
        }
    };
    let doc = match ws.documents.get(&document_url) {
        Some(doc) => doc,
        None => return,
    };

    collect_syntax_errors(doc, &mut diags);
    drop(workspaces);

    context
        .write_notification::<notification::PublishDiagnostics, _>(Some(PublishDiagnosticsParams {
            uri: document_url.clone(),
            diagnostics: diags.clone(),
            version: None,
        }))
        .await
        .unwrap_or_else(|err| tracing::error!("{err}"));

    if !diags.is_empty() {
        return;
    }

    let workspaces = context.workspaces.read().await;
    let ws = match workspaces.get(&ws_url) {
        Some(d) => d,
        None => {
            tracing::warn!(%document_url, "workspace not found");
            return;
        }
    };
    let doc = match ws.documents.get(&document_url) {
        Some(doc) => doc,
        None => return,
    };

    let dom = doc.dom.clone();

    collect_dom_errors(doc, &dom, &document_url, &mut diags);
    drop(workspaces);

    context
        .write_notification::<notification::PublishDiagnostics, _>(Some(PublishDiagnosticsParams {
            uri: document_url.clone(),
            diagnostics: diags.clone(),
            version: None,
        }))
        .await
        .unwrap_or_else(|err| tracing::error!("{err}"));

    if !diags.is_empty() {
        return;
    }

    let workspaces = context.workspaces.read().await;
    let ws = match workspaces.get(&ws_url) {
        Some(d) => d,
        None => {
            tracing::warn!(%document_url, "workspace not found");
            return;
        }
    };
    let doc = match ws.documents.get(&document_url) {
        Some(doc) => doc,
        None => return,
    };

    collect_schema_errors(ws, doc, &dom, &document_url, &mut diags).await;
    drop(workspaces);

    context
        .write_notification::<notification::PublishDiagnostics, _>(Some(PublishDiagnosticsParams {
            uri: document_url.clone(),
            diagnostics: diags.clone(),
            version: None,
        }))
        .await
        .unwrap_or_else(|err| tracing::error!("{err}"));
}

#[tracing::instrument(skip_all)]
pub(crate) async fn clear_diagnostics<E: Environment>(
    mut context: Context<World<E>>,
    document_url: Url,
) {
    context
        .write_notification::<notification::PublishDiagnostics, _>(Some(PublishDiagnosticsParams {
            uri: document_url,
            diagnostics: Vec::new(),
            version: None,
        }))
        .await
        .unwrap_or_else(|err| tracing::error!("{}", err));
}

#[tracing::instrument(skip_all)]
fn collect_syntax_errors(doc: &DocumentState, diags: &mut Vec<Diagnostic>) {
    diags.extend(doc.parse.errors.iter().map(|e| {
        let range = doc.mapper.range(e.range).unwrap_or_default().into_lsp();
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some(NAME.into()),
            message: e.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        }
    }));
}

#[tracing::instrument(skip_all)]
fn collect_dom_errors(
    doc: &DocumentState,
    dom: &Node,
    document_url: &Url,
    diags: &mut Vec<Diagnostic>,
) {
    if let Err(errors) = dom.validate() {
        for error in errors {
            match &error {
                jsona::dom::Error::ConflictingKeys { key, other } => {
                    let range = doc
                        .mapper
                        .range(key.text_range().unwrap())
                        .unwrap()
                        .into_lsp();

                    let other_range = doc
                        .mapper
                        .range(other.text_range().unwrap())
                        .unwrap()
                        .into_lsp();

                    diags.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some(NAME.into()),
                        message: error.to_string(),
                        related_information: Some(Vec::from([DiagnosticRelatedInformation {
                            location: Location {
                                uri: document_url.clone(),
                                range: other_range,
                            },
                            message: "other key defined here".into(),
                        }])),
                        ..Default::default()
                    });

                    diags.push(Diagnostic {
                        range: other_range,
                        severity: Some(DiagnosticSeverity::HINT),
                        source: Some(NAME.into()),
                        message: error.to_string(),
                        related_information: Some(Vec::from([DiagnosticRelatedInformation {
                            location: Location {
                                uri: document_url.clone(),
                                range,
                            },
                            message: "other key defined here".into(),
                        }])),
                        ..Default::default()
                    });
                }
                jsona::dom::Error::UnexpectedSyntax { syntax } => {
                    let range = doc
                        .mapper
                        .range(syntax.text_range())
                        .unwrap_or_default()
                        .into_lsp();
                    diags.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some(NAME.into()),
                        message: error.to_string(),
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
                jsona::dom::Error::InvalidEscapeSequence { syntax: _ }
                | jsona::dom::Error::InvalidNumber { syntax: _ } => {}
            }
        }
    }
}

#[tracing::instrument(skip_all, fields(%document_url))]
async fn collect_schema_errors<E: Environment>(
    ws: &WorkspaceState<E>,
    doc: &DocumentState,
    dom: &Node,
    document_url: &Url,
    diags: &mut Vec<Diagnostic>,
) {
    if !ws.config.schema.enabled {
        return;
    }

    if let Some(schema_association) = ws.schemas.associations().association_for(document_url) {
        tracing::debug!(
            schema.url = %schema_association.url,
            schema.name = schema_association.meta["name"].as_str().unwrap_or(""),
            schema.source = schema_association.meta["source"].as_str().unwrap_or(""),
            "using schema"
        );

        match ws.schemas.validate(&schema_association.url, dom).await {
            Ok(errors) => diags.extend(errors.into_iter().map(|err| {
                let range = err.node.node_text_range().unwrap_or_default();
                let range = doc.mapper.range(range).unwrap_or_default().into_lsp();
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some(NAME.into()),
                    message: err.info,
                    related_information: None,
                    tags: None,
                    data: None,
                }
            })),
            Err(error) => {
                tracing::error!(?error, "schema validation failed");
            }
        }
    }
}
