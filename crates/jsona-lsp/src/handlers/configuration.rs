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
use std::iter::once;

#[tracing::instrument(skip_all)]
pub async fn configuration_change<E: Environment>(
    context: Context<World<E>>,
    params: Params<DidChangeConfigurationParams>,
) {
    let p = match params.optional() {
        None => return,
        Some(p) => p,
    };

    let mut workspaces = context.workspaces.write().await;

    for (_, ws) in workspaces.iter_mut() {
        if let Err(error) = ws.initialize(context.clone(), &p.settings).await {
            tracing::error!(%error, "failed to update workspace");
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn update_configuration<E: Environment>(context: Context<World<E>>) {
    let mut workspaces = context.workspaces.write().await;

    let config_items: Vec<_> = workspaces
        .iter()
        .filter_map(|(uri, _)| {
            if *uri == *DEFAULT_WORKSPACE_URL {
                None
            } else {
                Some(ConfigurationItem {
                    scope_uri: Some(uri.clone()),
                    section: Some(DEFAULT_CONFIGURATION_SECTION.to_string()),
                })
            }
        })
        .collect();

    let res = context
        .clone()
        .write_request::<WorkspaceConfiguration, _>(Some(ConfigurationParams {
            items: once(ConfigurationItem {
                scope_uri: None,
                section: Some(DEFAULT_CONFIGURATION_SECTION.to_string()),
            })
            .chain(config_items.iter().cloned())
            .collect::<Vec<_>>(),
        }))
        .await
        .context("failed to fetch configuration")
        .and_then(|res| res.into_result().context("invalid configuration response"));

    match res {
        Ok(configs) => {
            for (i, config) in configs.into_iter().enumerate() {
                if config.is_object() {
                    if i == 0 {
                        for (_, ws) in workspaces
                            .iter_mut()
                            .filter(|(uri, _)| **uri == *DEFAULT_WORKSPACE_URL)
                        {
                            if let Err(error) = ws.initialize(context.clone(), &config).await {
                                let uri = DEFAULT_WORKSPACE_URL.as_str();
                                tracing::error!(%error, uri, "failed to update workspace");
                            }
                        }
                    } else {
                        let uri = config_items.get(i - 1).unwrap().scope_uri.as_ref().unwrap();
                        let ws = workspaces.get_mut(uri).unwrap();

                        if let Err(error) = ws.initialize(context.clone(), &config).await {
                            tracing::error!(%error, %uri, "failed to update workspace");
                        }
                    }
                }
            }
        }
        Err(error) => {
            tracing::error!(?error, "failed to fetch configuration");
        }
    }
}
