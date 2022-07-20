use anyhow::{anyhow, Context};
use itertools::Itertools;
use jsona_util::{config::Config, environment::Environment, schema::Schemas};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub use crate::commands::GeneralArgs;

pub mod commands;
pub mod printing;

pub struct App<E: Environment> {
    env: E,
    colors: bool,
    schemas: Schemas<E>,
    config: Option<Arc<Config>>,
}

impl<E: Environment> App<E> {
    pub fn new(env: E) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();

        #[cfg(target_arch = "wasm32")]
        let http = reqwest::Client::default();

        Self {
            schemas: Schemas::new(env.clone(), http),
            colors: env.atty_stderr(),
            config: None,
            env,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn load_config(&mut self, general: &GeneralArgs) -> Result<Arc<Config>, anyhow::Error> {
        if let Some(c) = self.config.clone() {
            return Ok(c);
        }

        let mut config_path = general.config.clone();

        if config_path.is_none() && !general.no_auto_config {
            if let Some(cwd) = self.env.cwd() {
                config_path = self.env.find_config_file(&cwd).await
            }
        }

        let mut config = Config::default();
        if let Some(c) = config_path {
            tracing::info!(path = ?c, "found configuration file");
            match self.env.read_file(&c).await {
                Ok(source) => {
                    match std::str::from_utf8(&source)
                        .map_err(|_| anyhow!("invalid utf8"))
                        .and_then(Config::from_jsona)
                    {
                        Ok(c) => config = c,
                        Err(error) => {
                            tracing::warn!(%error, "invalid configuration file");
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to read configuration file");
                }
            }
        }

        config
            .prepare(
                &self.env,
                &self
                    .env
                    .cwd()
                    .ok_or_else(|| anyhow!("working directory is required"))?,
            )
            .context("invalid configuration")?;

        let c = Arc::new(config);

        self.config = Some(c.clone());

        Ok(c)
    }

    #[tracing::instrument(skip_all, fields(?cwd))]
    async fn collect_files(
        &self,
        cwd: &Path,
        _config: &Config,
        arg_patterns: impl Iterator<Item = String>,
    ) -> Result<Vec<PathBuf>, anyhow::Error> {
        let mut patterns: Vec<String> = arg_patterns
            .map(|pat| {
                if !self.env.is_absolute(Path::new(&pat)) {
                    cwd.join(&pat).to_string_lossy().into_owned()
                } else {
                    pat
                }
            })
            .collect();

        if patterns.is_empty() {
            patterns = Vec::from([cwd.join("**/*.jsona").to_string_lossy().into_owned()])
        };

        let patterns = patterns
            .into_iter()
            .unique()
            .map(|p| glob::Pattern::new(&p).map(|_| p))
            .collect::<Result<Vec<_>, _>>()?;

        let files = patterns
            .into_iter()
            .map(|pat| self.env.glob_files(&pat))
            .collect::<Result<Vec<_>, _>>()
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();

        let total = files.len();

        let excluded = total - files.len();

        tracing::info!(total, excluded, "found files");

        Ok(files)
    }
}
