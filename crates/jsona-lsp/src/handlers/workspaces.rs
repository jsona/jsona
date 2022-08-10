use super::update_configuration;
use crate::world::{WorkspaceState, World};
use jsona_util::environment::Environment;
use lsp_async_stub::{Context, Params};
use lsp_types::DidChangeWorkspaceFoldersParams;

pub async fn workspace_change<E: Environment>(
    context: Context<World<E>>,
    params: Params<DidChangeWorkspaceFoldersParams>,
) {
    let p = match params.optional() {
        None => return,
        Some(p) => p,
    };

    let mut workspaces = context.workspaces.write().await;
    let init_config = context.init_config.load();

    for removed in p.event.removed {
        workspaces.remove(&removed.uri);
    }

    for added in p.event.added {
        let ws = workspaces
            .entry(added.uri.clone())
            .or_insert(WorkspaceState::new(context.env.clone(), added.uri));

        ws.schemas.set_cache_path(init_config.cache_path.clone());

        if let Err(error) = ws.initialize(context.clone(), &context.env).await {
            tracing::error!(?error, "failed to initialize workspace");
        }
    }

    drop(workspaces);
    update_configuration(context).await;
}
