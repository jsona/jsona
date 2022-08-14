use crate::{
    config::{InitializationOptions, LspConfig},
    lsp_ext::notification::{InitializeWorkspace, InitializeWorkspaceParams},
};
use anyhow::anyhow;
use arc_swap::ArcSwap;
use jsona::{
    dom::{Keys, Node},
    parser::Parse,
};
use jsona_schema::Schema;
use jsona_util::{
    config::Config,
    environment::Environment,
    schema::{
        associations::{priority, source, AssociationRule, SchemaAssociation},
        Schemas,
    },
    util::to_file_path,
    AsyncRwLock, HashMap, IndexMap,
};
use lsp_async_stub::{rpc, util::Mapper, Context, RequestWriter};
use lsp_types::Url;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

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
    pub fn by_document(&self, url: &Url) -> &WorkspaceState<E> {
        self.0
            .iter()
            .filter(|(key, _)| url.as_str().starts_with(key.as_str()))
            .max_by(|(a, _), (b, _)| a.as_str().len().cmp(&b.as_str().len()))
            .map_or_else(
                || {
                    tracing::warn!(document_url = %url, "using detached workspace");
                    self.0.get(&*DEFAULT_WORKSPACE_URL).unwrap()
                },
                |(_, ws)| ws,
            )
    }

    pub fn by_document_mut(&mut self, url: &Url) -> &mut WorkspaceState<E> {
        self.0
            .iter_mut()
            .filter(|(key, _)| {
                url.as_str().starts_with(key.as_str()) || *key == &*DEFAULT_WORKSPACE_URL
            })
            .max_by(|(a, _), (b, _)| a.as_str().len().cmp(&b.as_str().len()))
            .map(|(k, ws)| {
                if k == &*DEFAULT_WORKSPACE_URL {
                    tracing::warn!(document_url = %url, "using detached workspace");
                }

                ws
            })
            .unwrap()
    }
}

pub struct WorldState<E: Environment> {
    pub(crate) env: E,
    pub(crate) id: String,
    pub(crate) workspaces: AsyncRwLock<Workspaces<E>>,
    pub(crate) initialization_options: ArcSwap<InitializationOptions>,
    pub(crate) default_config: ArcSwap<Config>,
}

pub static DEFAULT_WORKSPACE_URL: Lazy<Url> = Lazy::new(|| Url::parse("root:///").unwrap());

impl<E: Environment> WorldState<E> {
    pub fn new(env: E) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workspaces: {
                let mut m = IndexMap::default();
                m.insert(
                    DEFAULT_WORKSPACE_URL.clone(),
                    WorkspaceState::new(env.clone(), DEFAULT_WORKSPACE_URL.clone()),
                );
                AsyncRwLock::new(Workspaces(m))
            },
            initialization_options: Default::default(),
            default_config: Default::default(),
            env,
        }
    }

    /// Set the world state's default config.
    pub fn set_default_config(&self, default_config: Arc<Config>) {
        self.default_config.store(default_config);
    }
}

pub struct WorkspaceState<E: Environment> {
    pub(crate) root: Url,
    pub(crate) documents: HashMap<lsp_types::Url, DocumentState>,
    pub(crate) jsona_config: Config,
    pub(crate) schemas: Schemas<E>,
    pub(crate) lsp_config: LspConfig,
}

impl<E: Environment> WorkspaceState<E> {
    pub(crate) fn new(env: E, root: Url) -> Self {
        Self {
            root,
            documents: Default::default(),
            jsona_config: Default::default(),
            schemas: Schemas::new(env),
            lsp_config: LspConfig::default(),
        }
    }
}

impl<E: Environment> WorkspaceState<E> {
    pub(crate) fn document(&self, url: &Url) -> Result<&DocumentState, rpc::Error> {
        self.documents
            .get(url)
            .ok_or_else(rpc::Error::invalid_params)
    }

    #[tracing::instrument(skip_all, fields(%self.root))]
    pub(crate) async fn initialize(
        &mut self,
        context: Context<World<E>>,
        lsp_config: &Value,
    ) -> Result<(), anyhow::Error> {
        self.schemas
            .set_cache_path(context.initialization_options.load().cache_path.clone());

        if let Err(error) = self.lsp_config.update_from_json(lsp_config) {
            tracing::error!(?error, "invalid configuration");
        }

        self.load_config(&context.env, &*context.world().default_config.load())
            .await?;

        self.schemas
            .associations()
            .retain(|(_, assoc)| assoc.meta["source"] == "manual");

        if !self.lsp_config.schema.enabled {
            return Ok(());
        }

        self.schemas
            .associations()
            .add_from_config(&self.jsona_config);

        for (schema_url, file_urls) in &self.lsp_config.schema.associations {
            if let Ok(schema_url) = schema_url.parse() {
                let assoc = SchemaAssociation {
                    url: schema_url,
                    meta: json!({
                        "source": source::LSP_CONFIG,
                    }),
                    priority: priority::LSP_CONFIG,
                };
                for file_url in file_urls {
                    if let Ok(file_url) = file_url.parse() {
                        self.schemas
                            .associations()
                            .add(AssociationRule::Url(file_url), assoc.clone())
                    }
                }
            }
        }
        let store_url = &self.lsp_config.schema.schemastore;

        if let Err(error) = self
            .schemas
            .associations()
            .add_from_schemastore(store_url)
            .await
        {
            tracing::error!(%error, url=%store_url, "failed to set schemastore");
        }

        self.emit_initialize_workspace(context.clone()).await;

        Ok(())
    }

    pub(crate) async fn load_config(
        &mut self,
        env: &impl Environment,
        default_config: &Config,
    ) -> Result<(), anyhow::Error> {
        if !self.lsp_config.config_file.enabled {
            self.jsona_config = default_config.clone();
            return Ok(());
        }

        if self.root == *DEFAULT_WORKSPACE_URL {
            return Ok(());
        };

        let mut config_path = self.lsp_config.config_file.path.clone();
        if let Some(path) = config_path.as_ref() {
            if path.as_os_str().is_empty() {
                config_path = None;
            }
        }

        if let Some(path) = config_path.as_ref() {
            tracing::info!(path = ?path, "read config file");
            match Config::from_file(path, env).await {
                Ok(config) => {
                    self.jsona_config = config;
                }
                Err(err) => {
                    config_path = None;
                    tracing::error!("failed to read config {}", err);
                }
            }
        } else {
            let root_path =
                PathBuf::from(to_file_path(&self.root).ok_or_else(|| anyhow!("invalid root URL"))?);

            match Config::find_and_load(&root_path, env).await {
                Ok((path, config)) => {
                    tracing::info!(path = ?path, "found config file");
                    self.jsona_config = config;
                    config_path = Some(path);
                }
                Err(err) => {
                    tracing::error!("failed to load config {}", err);
                }
            }
        }

        self.jsona_config.prepare(config_path)?;
        tracing::debug!("using config: {:#?}", self.jsona_config);

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

    #[tracing::instrument(skip_all, fields(%file, %path))]
    pub(crate) async fn schemas_at_path(&self, file: &Url, path: &Keys) -> Option<Vec<Schema>> {
        let schema_association = self.schemas.associations().association_for(file)?;
        match self
            .schemas
            .schemas_at_path(&schema_association.url, path)
            .await
        {
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
