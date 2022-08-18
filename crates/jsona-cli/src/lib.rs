use jsona_util::{environment::Environment, schema::Schemas};
use url::Url;

pub use crate::commands::{AppArgs, Colors, GeneralArgs};

use anyhow::anyhow;
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
    pub async fn load_file(&self, path: &str) -> Result<(Url, String), anyhow::Error> {
        let url = self
            .env
            .to_file_uri(path)
            .ok_or_else(|| anyhow!("invalid file path"))?;
        let data = self.env.read_file(&url).await?;
        let content = std::str::from_utf8(&data).map_err(|_| anyhow!("invalid utf8 content"))?;
        Ok((url, content.to_string()))
    }
}
