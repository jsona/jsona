use crate::{
    lsp_ext::request::{
        AssociatedSchemaParams, AssociatedSchemaResponse, ListSchemasParams, ListSchemasResponse,
        SchemaInfo,
    },
    world::World,
};
use jsona_util::{environment::Environment, schema::associations::AssociationRule};
use lsp_async_stub::{rpc::Error, Context, Params};

#[tracing::instrument(skip_all)]
pub async fn list_schemas<E: Environment>(
    context: Context<World<E>>,
    params: Params<ListSchemasParams>,
) -> Result<ListSchemasResponse, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let ws = workspaces.by_document(&p.document_uri);

    let associations = ws.schemas.associations().read();

    Ok(ListSchemasResponse {
        schemas: associations
            .iter()
            .filter(|(rule, _)| !matches!(rule, AssociationRule::Url(..)))
            .map(|(_, s)| SchemaInfo {
                url: s.url.clone(),
                meta: s.meta.clone(),
            })
            .collect(),
    })
}

#[tracing::instrument(skip_all)]
pub async fn associated_schema<E: Environment>(
    context: Context<World<E>>,
    params: Params<AssociatedSchemaParams>,
) -> Result<AssociatedSchemaResponse, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let ws = workspaces.by_document(&p.document_uri);

    Ok(AssociatedSchemaResponse {
        schema: ws
            .schemas
            .associations()
            .association_for(&p.document_uri)
            .map(|s| SchemaInfo {
                url: s.url.clone(),
                meta: s.meta.clone(),
            }),
    })
}
