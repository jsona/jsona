use anyhow::Context;
use jsona_util::{config::Config, environment::Environment, schema::Schemas, util::path_utils};
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

        let mut config = Config::default();
        if config_path.is_none() && !general.no_auto_config {
            if let Some(cwd) = self.env.cwd() {
                match Config::find_and_load(&cwd, &self.env).await {
                    Ok((path, c)) => {
                        tracing::info!(path = ?config_path, "found configuration file");
                        config_path = Some(path);
                        config = c;
                    }
                    Err(error) => {
                        tracing::warn!(%error, "failed to load configuration file");
                    }
                }
            }
        } else if let Some(config_path) = config_path.as_mut() {
            *config_path = PathBuf::from(path_utils::join_path(
                config_path.display().to_string(),
                &self.env.cwd().unwrap_or_else(|| PathBuf::from("/")),
            ));
            tracing::info!(path = ?config_path, "found configuration file");
            match Config::from_file(config_path, &self.env).await {
                Ok(c) => {
                    config = c;
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to read configuration file");
                }
            }
        }
        config
            .prepare(config_path)
            .context("invalid configuration")?;

        tracing::debug!("using config: {:#?}", config);

        let c = Arc::new(config);

        self.config = Some(c.clone());

        Ok(c)
    }

    #[tracing::instrument(skip_all, fields(?cwd))]
    async fn collect_files(
        &self,
        cwd: &Path,
        config: &Config,
        arg_patterns: impl Iterator<Item = String>,
    ) -> Result<Vec<PathBuf>, anyhow::Error> {
        let mut patterns: Vec<String> = arg_patterns
            .map(|pat| {
                if !path_utils::is_absolute(&pat) {
                    path_utils::join_path(&pat, cwd)
                } else {
                    pat
                }
            })
            .collect();

        if patterns.is_empty() {
            patterns = vec![path_utils::to_unix(path_utils::join_path(
                "**/*.jsona",
                cwd,
            ))];
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

        let files = files
            .into_iter()
            .filter(|path| config.is_included(path.display().to_string()))
            .collect::<Vec<_>>();

        let excluded = total - files.len();

        tracing::info!(total, excluded, "found files");

        Ok(files)
    }
}
