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

    for removed in p.event.removed {
        workspaces.remove(&removed.uri);
    }

    for added in p.event.added {
        let ws = workspaces
            .entry(added.uri.clone())
            .or_insert(WorkspaceState::new(context.env.clone(), added.uri));

        if let Err(error) = ws.initialize(context.clone(), &context.env).await {
            tracing::error!(?error, "failed to initialize workspace");
        }
    }

    drop(workspaces);
    update_configuration(context).await;
}
