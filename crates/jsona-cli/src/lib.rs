use anyhow::{anyhow, Context};
use jsona_util::{config::Config, environment::Environment, schema::Schemas};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub use crate::commands::{AppArgs, Colors, GeneralArgs};

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
        Self {
            schemas: Schemas::new(env.clone()),
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
        if let Some(config_path) = config_path {
            tracing::info!(path = ?config_path, "found configuration file");
            match self.env.read_file(&config_path).await {
                Ok(source) => {
                    match std::str::from_utf8(&source)
                        .map_err(|_| anyhow!("invalid utf8"))
                        .and_then(Config::from_jsona)
                    {
                        Ok(c) => {
                            config = c;
                            config
                                .prepare(&config_path)
                                .context("invalid configuration")?;
                        }
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
