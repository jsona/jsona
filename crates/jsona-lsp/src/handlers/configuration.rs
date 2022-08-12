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
        if let Err(error) = ws.config.update_from_json(&p.settings) {
            tracing::error!(?error, "invalid configuration");
        }

        if let Err(error) = ws.initialize(context.clone(), &context.env).await {
            tracing::error!(%error, "failed to update workspace");
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn update_configuration<E: Environment>(context: Context<World<E>>) {
    let mut workspaces = context.workspaces.write().await;

    let config_items: Vec<_> = workspaces
        .iter()
        .filter_map(|(url, _)| {
            if *url == *DEFAULT_WORKSPACE_URL {
                None
            } else {
                Some(ConfigurationItem {
                    scope_uri: Some(url.clone()),
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
                if i == 0 && config.is_object() {
                    for (_, ws) in workspaces
                        .iter_mut()
                        .filter(|(url, _)| **url == *DEFAULT_WORKSPACE_URL)
                    {
                        if let Err(error) = ws.config.update_from_json(&config) {
                            tracing::error!(?error, "invalid configuration");
                        }

                        if let Err(error) = ws.initialize(context.clone(), &context.env).await {
                            tracing::error!(%error, "failed to update workspace");
                        }
                    }
                } else if config.is_object() {
                    let ws_url = config_items.get(i - 1).unwrap().scope_uri.as_ref().unwrap();
                    let ws = workspaces.get_mut(ws_url).unwrap();
                    if let Err(error) = ws.config.update_from_json(&config) {
                        tracing::error!(?error, "invalid configuration");
                    }

                    if let Err(error) = ws.initialize(context.clone(), &context.env).await {
                        tracing::error!(%error, "failed to update workspace");
                    }
                }
            }
        }
        Err(error) => {
            tracing::error!(?error, "failed to fetch configuration");
        }
    }
}
