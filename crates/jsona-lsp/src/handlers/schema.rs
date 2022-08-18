use crate::{
    lsp_ext::request::{
        AssociatedSchemaParams, AssociatedSchemaResponse, ListSchemasParams, ListSchemasResponse,
        SchemaInfo,
    },
    world::World,
};
use indexmap::IndexMap;
use jsona_util::{environment::Environment, schema::associations::priority};
use lsp_async_stub::{rpc::Error, Context, Params};
use serde_json::Value;
use url::Url;

#[tracing::instrument(skip_all)]
pub async fn list_schemas<E: Environment>(
    context: Context<World<E>>,
    params: Params<ListSchemasParams>,
) -> Result<ListSchemasResponse, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let document_uri = p.document_uri;
    let ws = workspaces.by_document(&document_uri);

    let associations = ws.schemas.associations().read();
    let mut map: IndexMap<Url, Value> = associations
        .iter()
        .filter(|(_, s)| matches!(s.priority, priority::STORE))
        .map(|(_, s)| (s.url.clone(), s.meta.clone()))
        .collect();
    for (_, s) in associations
        .iter()
        .filter(|(_, s)| s.priority != priority::STORE)
    {
        if map.contains_key(&s.url) {
            continue;
        }
        map.insert(s.url.clone(), s.meta.clone());
    }

    Ok(ListSchemasResponse {
        schemas: map
            .into_iter()
            .map(|(url, meta)| SchemaInfo { url, meta })
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
    let document_uri = p.document_uri;
    let ws = workspaces.by_document(&document_uri);

    Ok(AssociatedSchemaResponse {
        schema: ws
            .schemas
            .associations()
            .query_for(&document_uri)
            .map(|s| SchemaInfo {
                url: s.url.clone(),
                meta: s.meta.clone(),
            }),
    })
}
