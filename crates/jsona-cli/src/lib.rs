use jsona_util::{environment::Environment, schema::Schemas, util::path_utils};
use std::path::{Path, PathBuf};

pub use crate::commands::{AppArgs, Colors, GeneralArgs};

pub mod commands;
pub mod printing;

pub struct App<E: Environment> {
    env: E,
    colors: bool,
    schemas: Schemas<E>,
}

impl<E: Environment> App<E> {
    pub fn new(env: E) -> Self {
        Self {
            schemas: Schemas::new(env.clone()),
            colors: env.atty_stderr(),
            env,
        }
    }

    fn collect_files(
        &self,
        cwd: &Path,
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

        Ok(files)
    }
}
