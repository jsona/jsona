use async_trait::async_trait;
use time::OffsetDateTime;
use url::Url;

use anyhow::anyhow;
use std::path::PathBuf;

use super::Environment;
use crate::util::url::{to_file_path, to_url};

#[derive(Clone)]
pub struct NativeEnvironment {
    handle: tokio::runtime::Handle,
}

impl NativeEnvironment {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handle: tokio::runtime::Handle::current(),
        }
    }
}

impl Default for NativeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Environment for NativeEnvironment {
    type Stdin = tokio::io::Stdin;
    type Stdout = tokio::io::Stdout;
    type Stderr = tokio::io::Stderr;

    fn now(&self) -> time::OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn spawn<F>(&self, fut: F)
    where
        F: futures::Future + Send + 'static,
        F::Output: Send,
    {
        self.handle.spawn(fut);
    }

    fn spawn_local<F>(&self, fut: F)
    where
        F: futures::Future + 'static,
    {
        tokio::task::spawn_local(fut);
    }

    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn atty_stderr(&self) -> bool {
        atty::is(atty::Stream::Stderr)
    }

    fn stdin(&self) -> Self::Stdin {
        tokio::io::stdin()
    }

    fn stdout(&self) -> Self::Stdout {
        tokio::io::stdout()
    }

    fn stderr(&self) -> Self::Stderr {
        tokio::io::stderr()
    }

    async fn read_file(&self, path: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let path = to_file_path(path).ok_or_else(|| anyhow!("failed to read file at ${path}"))?;
        Ok(tokio::fs::read(PathBuf::from(path)).await?)
    }

    async fn write_file(&self, path: &Url, bytes: &[u8]) -> Result<(), anyhow::Error> {
        let path = to_file_path(path).ok_or_else(|| anyhow!("failed to read file at ${path}"))?;
        Ok(tokio::fs::write(PathBuf::from(path), bytes).await?)
    }

    #[cfg(feature = "fetch")]
    async fn fetch_file(&self, path: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        let data = client
            .get(path.clone())
            .send()
            .await?
            .bytes()
            .await?
            .to_vec();
        Ok(data)
    }

    #[cfg(not(feature = "fetch"))]
    async fn fetch_file(&self, url: &Url) -> Result<Vec<u8>, anyhow::Error> {
        anyhow::bail!("failed to fetch `{url}`, fetch is not supported")
    }

    fn root_uri(&self) -> Option<Url> {
        let cwd = std::env::current_dir().ok()?;
        to_url(&cwd.display().to_string(), &None)
    }
}
