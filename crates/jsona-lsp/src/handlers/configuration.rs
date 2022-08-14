use crate::{
    config::DEFAULT_CONFIGURATION_SECTION,
    world::{World, DEFAULT_WORKSPACE_URL},
};
use anyhow::Context as AnyhowContext;
use jsona_util::environment::Environment;
use lsp_async_stub::{Context, Params, RequestWriter};
use lsp_types::{
    request::WorkspaceConfiguration, ConfigurationItem, ConfigurationParams,
    DidChangeConfigurationParams,
};
use serde_json::Value;
use url::Url;

#[tracing::instrument(skip_all)]
pub async fn configuration_change<E: Environment>(
    context: Context<World<E>>,
    params: Params<DidChangeConfigurationParams>,
) {
    match params.optional() {
        None => return,
        Some(p) => p,
    };
    update_configuration(context).await;
}

#[tracing::instrument(skip_all)]
pub async fn update_configuration<E: Environment>(context: Context<World<E>>) {
    let workspaces = context.workspaces.read().await;
    let urls: Vec<&Url> = workspaces.keys().collect();
    let items: Vec<_> = urls
        .clone()
        .into_iter()
        .map(|uri| {
            let scope_uri = if *uri == *DEFAULT_WORKSPACE_URL {
                Some(DEFAULT_WORKSPACE_URL.clone())
            } else {
                Some(uri.clone())
            };
            ConfigurationItem {
                scope_uri,
                section: Some(DEFAULT_CONFIGURATION_SECTION.to_string()),
            }
        })
        .collect();

    let res = context
        .clone()
        .write_request::<WorkspaceConfiguration, _>(Some(ConfigurationParams { items }))
        .await
        .context("failed to fetch configuration")
        .and_then(|res| res.into_result().context("invalid configuration response"));

    match res {
        Ok(configs) => {
            for (config, uri) in configs.into_iter().zip(urls) {
                if config.is_object() {
                    context.env.spawn_local(initialize_workspace(
                        context.clone(),
                        uri.clone(),
                        config.clone(),
                    ));
                }
            }
        }
        Err(error) => {
            tracing::error!(?error, "failed to fetch configuration");
        }
    }
}

pub async fn initialize_workspace<E: Environment>(
    context: Context<World<E>>,
    uri: Url,
    config: Value,
) {
    let mut workspaces = context.workspaces.write().await;
    if let Some(ws) = workspaces.get_mut(&uri) {
        if let Err(error) = ws.initialize(context.clone(), &config).await {
            tracing::error!(%error, %uri, "failed to update workspace");
        }
    }
}
