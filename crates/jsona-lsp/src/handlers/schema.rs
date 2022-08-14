use crate::{
    lsp_ext::{
        notification::{self, AssociateSchemasParams},
        request::{
            AssociatedSchemaParams, AssociatedSchemaResponse, ListSchemasParams,
            ListSchemasResponse, SchemaInfo,
        },
    },
    world::World,
};
use jsona_util::{
    environment::Environment,
    schema::associations::{priority, source, AssociationRule, SchemaAssociation},
};
use lsp_async_stub::{rpc::Error, Context, Params};
use serde_json::json;

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
pub async fn associate_schemas<E: Environment>(
    context: Context<World<E>>,
    params: Params<AssociateSchemasParams>,
) {
    let p = match params.required() {
        Ok(p) => p,
        Err(_) => return,
    };

    let workspaces = context.workspaces.read().await;

    for (_, ws) in workspaces.iter() {
        for item in &p.associations {
            let assoc = SchemaAssociation {
                priority: priority::EXT_CONFIG,
                url: item.schema_uri.clone(),
                meta: {
                    let mut meta = item.meta.clone().unwrap_or_default();
                    if !meta.is_object() {
                        meta = json!({});
                    }
                    meta["source"] = source::EXT_CONFIG.into();
                    meta
                },
            };
            // FIXME: there is no way to remove these.
            match &item.rule {
                notification::AssociationRule::Glob(glob) => {
                    let rule = match AssociationRule::glob(glob) {
                        Ok(re) => re,
                        Err(err) => {
                            tracing::error!(
							error = %err,
							schema_uri = %assoc.url,
							"invalid pattern for schema");
                            return;
                        }
                    };

                    ws.schemas.associations().add(rule, assoc.clone());
                }
                notification::AssociationRule::Regex(regex) => {
                    let rule = match AssociationRule::regex(regex) {
                        Ok(re) => re,
                        Err(err) => {
                            tracing::error!(
							error = %err,
							schema_uri = %assoc.url,
							"invalid pattern for schema");
                            return;
                        }
                    };

                    ws.schemas.associations().add(rule, assoc.clone());
                }
                notification::AssociationRule::Url(url) => {
                    ws.schemas.associations().add(url.into(), assoc.clone());
                }
            };
        }
    }
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
