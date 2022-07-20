use jsona_util::environment::Environment;
use lsp_async_stub::{rpc::Error, Context, Params};
use lsp_types::{Hover, HoverParams};

use crate::World;

#[tracing::instrument(skip_all)]
pub(crate) async fn hover<E: Environment>(
    _context: Context<World<E>>,
    _params: Params<HoverParams>,
) -> Result<Option<Hover>, Error> {
    Ok(None)
}
