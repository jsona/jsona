use std::sync::Arc;

use super::update_configuration;
use crate::config::InitConfig;
use crate::world::WorkspaceState;
use crate::World;
use jsona_util::environment::Environment;
use lsp_async_stub::{rpc::Error, Context, Params};
use lsp_types::{
    CompletionOptions, FoldingRangeProviderCapability, HoverProviderCapability, InitializedParams,
    OneOf, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
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
        match serde_json::from_value::<InitConfig>(init_opts) {
            Ok(c) => context.init_config.store(Arc::new(c)),
            Err(error) => {
                tracing::error!(%error, "invalid initialization options");
            }
        }
    }

    if let Some(workspaces) = p.workspace_folders {
        let mut wss = context.workspaces.write().await;
        let init_config = context.init_config.load();

        for workspace in workspaces {
            let ws = wss
                .entry(workspace.uri.clone())
                .or_insert(WorkspaceState::new(context.env.clone(), workspace.uri));

            ws.schemas
                .cache()
                .set_cache_path(init_config.cache_path.clone());

            if let Err(error) = ws.initialize(context.clone(), &context.env).await {
                tracing::error!(?error, "failed to initialize workspace");
            }
        }
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
            document_symbol_provider: Some(OneOf::Left(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![
                    ":".into(),
                    "[".into(),
                    "{".into(),
                    ",".into(),
                    "@".into(),
                ]),
                ..Default::default()
            }),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: "Jsona".into(),
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
    context
        .env
        .spawn_local(update_configuration(context.clone()));
}
