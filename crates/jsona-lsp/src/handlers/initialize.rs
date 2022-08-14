use std::sync::Arc;

use super::update_configuration;
use crate::config::InitializationOptions;
use crate::world::{WorkspaceState, DEFAULT_WORKSPACE_URI};
use crate::World;
use jsona_util::environment::Environment;
use lsp_async_stub::{rpc::Error, Context, Params, RequestWriter};
use lsp_types::notification::{DidChangeConfiguration, Notification};
use lsp_types::request::RegisterCapability;
use lsp_types::{
    CompletionOptions, FoldingRangeProviderCapability, HoverProviderCapability, InitializedParams,
    OneOf, Registration, RegistrationParams, SelectionRangeProviderCapability, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};
use lsp_types::{InitializeParams, InitializeResult};

#[tracing::instrument(skip_all)]
pub async fn initialize<E: Environment>(
    context: Context<World<E>>,
    params: Params<InitializeParams>,
) -> Result<InitializeResult, Error> {
    let p = params.required()?;

    if let Some(init_opts) = p.initialization_options {
        match serde_json::from_value::<InitializationOptions>(init_opts) {
            Ok(c) => {
                tracing::debug!("use initialization options {:?}", c);
                context.initialization_options.store(Arc::new(c))
            }
            Err(error) => {
                tracing::error!(%error, "invalid initialization options");
            }
        }
    }

    if let Some(workspaces) = p.workspace_folders {
        let mut wss = context.workspaces.write().await;
        for workspace in workspaces {
            wss.entry(workspace.uri.clone())
                .or_insert(WorkspaceState::new(context.env.clone(), workspace.uri));
        }
    } else {
        let mut wss = context.workspaces.write().await;
        wss.entry(DEFAULT_WORKSPACE_URI.clone())
            .or_insert(WorkspaceState::new(
                context.env.clone(),
                DEFAULT_WORKSPACE_URI.clone(),
            ));
    }

    Ok(InitializeResult {
        capabilities: ServerCapabilities {
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                ..Default::default()
            }),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
            selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![":".into(), "(".into(), "@".into()]),
                ..Default::default()
            }),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: "JSONA".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
        offset_encoding: None,
    })
}

#[tracing::instrument(skip_all)]
pub async fn initialized<E: Environment>(
    context: Context<World<E>>,
    _params: Params<InitializedParams>,
) {
    if let Err(error) = context
        .clone()
        .write_request::<RegisterCapability, _>(Some(RegistrationParams {
            registrations: vec![Registration {
                id: context.id.clone(),
                method: DidChangeConfiguration::METHOD.into(),
                register_options: None,
            }],
        }))
        .await
    {
        tracing::error!(?error, "failed to send registration");
    }
    context
        .env
        .spawn_local(update_configuration(context.clone()));
}
