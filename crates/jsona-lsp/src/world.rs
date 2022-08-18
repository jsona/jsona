use crate::{
    config::{InitializationOptions, LspConfig},
    lsp_ext::notification::{InitializeWorkspace, InitializeWorkspaceParams},
};
use arc_swap::ArcSwap;
use jsona::{
    dom::{Keys, Node},
    parser::Parse,
};
use jsona_schema::Schema;
use jsona_util::{
    environment::Environment,
    schema::{
        associations::{priority, source, AssociationRule, SchemaAssociation},
        Schemas,
    },
    AsyncRwLock, HashMap, IndexMap,
};
use lsp_async_stub::{rpc, util::Mapper, Context, RequestWriter};
use lsp_types::Url;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::sync::Arc;
pub type World<E> = Arc<WorldState<E>>;

#[repr(transparent)]
pub struct Workspaces<E: Environment>(IndexMap<Url, WorkspaceState<E>>);

impl<E: Environment> std::ops::Deref for Workspaces<E> {
    type Target = IndexMap<Url, WorkspaceState<E>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E: Environment> std::ops::DerefMut for Workspaces<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<E: Environment> Workspaces<E> {
    pub fn by_document(&self, document_uri: &Url) -> &WorkspaceState<E> {
        self.0
            .iter()
            .filter(|(key, _)| {
                document_uri.as_str().starts_with(key.as_str()) || *key == &*DEFAULT_WORKSPACE_URI
            })
            .max_by(|(a, _), (b, _)| a.as_str().len().cmp(&b.as_str().len()))
            .map(|(_, ws)| ws)
            .unwrap()
    }

    pub fn by_document_mut(&mut self, url: &Url) -> &mut WorkspaceState<E> {
        self.0
            .iter_mut()
            .filter(|(key, _)| {
                url.as_str().starts_with(key.as_str()) || *key == &*DEFAULT_WORKSPACE_URI
            })
            .max_by(|(a, _), (b, _)| a.as_str().len().cmp(&b.as_str().len()))
            .map(|(_, ws)| ws)
            .unwrap()
    }

    pub fn try_get_document(
        &self,
        document_uri: &Url,
    ) -> Result<(&WorkspaceState<E>, &DocumentState), rpc::Error> {
        let ws = self.by_document(document_uri);
        let doc = ws.try_get_document(document_uri)?;
        Ok((ws, doc))
    }
}

pub struct WorldState<E: Environment> {
    pub(crate) env: E,
    pub(crate) id: String,
    pub(crate) workspaces: AsyncRwLock<Workspaces<E>>,
    pub(crate) initialization_options: ArcSwap<InitializationOptions>,
}

pub static DEFAULT_WORKSPACE_URI: Lazy<Url> = Lazy::new(|| Url::parse("root:///").unwrap());

impl<E: Environment> WorldState<E> {
    pub fn new(env: E) -> Self {
        let id = format!(
            "{:x}",
            md5::compute(format!("JSONA-{}", env.now().unix_timestamp_nanos()))
        );
        Self {
            id,
            workspaces: {
                let mut m = IndexMap::default();
                m.insert(
                    DEFAULT_WORKSPACE_URI.clone(),
                    WorkspaceState::new(env.clone(), DEFAULT_WORKSPACE_URI.clone()),
                );
                AsyncRwLock::new(Workspaces(m))
            },
            initialization_options: Default::default(),
            env,
        }
    }
}

pub struct WorkspaceState<E: Environment> {
    pub(crate) root: Url,
    pub(crate) documents: HashMap<lsp_types::Url, DocumentState>,
    pub(crate) schemas: Schemas<E>,
    pub(crate) lsp_config: LspConfig,
}

impl<E: Environment> WorkspaceState<E> {
    pub(crate) fn new(env: E, root: Url) -> Self {
        Self {
            root,
            documents: Default::default(),
            schemas: Schemas::new(env),
            lsp_config: LspConfig::default(),
        }
    }
}

impl<E: Environment> WorkspaceState<E> {
    pub(crate) fn try_get_document(
        &self,
        document_uri: &Url,
    ) -> Result<&DocumentState, rpc::Error> {
        self.documents.get(document_uri).ok_or_else(|| {
            tracing::debug!(%document_uri, "not found document in workspace");
            rpc::Error::invalid_params()
        })
    }

    #[tracing::instrument(skip_all, fields(%self.root))]
    pub(crate) async fn initialize(
        &mut self,
        context: Context<World<E>>,
        lsp_config: &Value,
    ) -> Result<(), anyhow::Error> {
        if let Err(error) = self.lsp_config.update_from_json(lsp_config) {
            tracing::error!(?error, "invalid configuration");
        }

        if self.lsp_config.schema.cache {
            let cache_path = context
                .initialization_options
                .load()
                .cache_path
                .as_ref()
                .and_then(|v| context.env.to_file_uri(v));
            self.schemas.set_cache_path(cache_path);
        } else {
            self.schemas.set_cache_path(None);
        }

        self.schemas.associations().clear();

        if !self.lsp_config.schema.enabled {
            return Ok(());
        }

        let store_url = self.lsp_config.schema.store_url.clone();

        if let Err(error) = self
            .schemas
            .associations()
            .add_from_schemastore(&store_url, &Some(self.root.clone()))
            .await
        {
            tracing::error!(%error, url=?store_url.map(|v| v.to_string()), "failed to load schemastore");
        }

        for (name, items) in &self.lsp_config.schema.associations {
            match self.schemas.associations().to_schema_url(name) {
                Some(schema_uri) => {
                    let assoc = SchemaAssociation {
                        url: schema_uri.clone(),
                        meta: json!({
                            "source": source::LSP_CONFIG,
                        }),
                        priority: priority::LSP_CONFIG,
                    };
                    match AssociationRule::batch(items, &Some(self.root.clone())) {
                        Ok(rules) => {
                            for rule in rules {
                                self.schemas.associations().add(rule, assoc.clone())
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, %schema_uri, "failed to add schema associations");
                        }
                    }
                }
                None => {
                    tracing::error!(%name, "failed to add schema associations");
                }
            }
        }

        self.emit_initialize_workspace(context.clone()).await;

        Ok(())
    }

    pub(crate) async fn emit_initialize_workspace(&self, mut context: Context<World<E>>) {
        if let Err(error) = context
            .write_notification::<InitializeWorkspace, _>(Some(InitializeWorkspaceParams {
                root_uri: self.root.clone(),
            }))
            .await
        {
            tracing::error!(%error, "failed to write notification");
        }
    }

    pub(crate) async fn query_schemas(&self, file: &Url, path: &Keys) -> Option<Vec<Schema>> {
        let schema_association = self.schemas.associations().query_for(file)?;
        match self.schemas.query(&schema_association.url, path).await {
            Ok(v) => Some(v),
            Err(error) => {
                tracing::error!(?error, "failed to query schemas");
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocumentState {
    pub(crate) parse: Parse,
    pub(crate) dom: Node,
    pub(crate) mapper: Mapper,
}
