use std::path::Path;

use super::Environment;
use async_trait::async_trait;
use time::OffsetDateTime;

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

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, anyhow::Error> {
        Ok(tokio::fs::read(path).await?)
    }

    async fn write_file(&self, path: &std::path::Path, bytes: &[u8]) -> Result<(), anyhow::Error> {
        Ok(tokio::fs::write(path, bytes).await?)
    }

    #[cfg(feature = "fetch")]
    async fn fetch_file(&self, url: &url::Url) -> Result<Vec<u8>, anyhow::Error> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        let data = client
            .get(url.clone())
            .send()
            .await?
            .bytes()
            .await?
            .to_vec();
        Ok(data)
    }

    #[cfg(not(feature = "fetch"))]
    async fn fetch_file(&self, url: &url::Url) -> Result<Vec<u8>, anyhow::Error> {
        anyhow::bail!("failed to fetch `{url}`, fetch is not supported")
    }

    fn cwd(&self) -> Option<std::path::PathBuf> {
        std::env::current_dir().ok()
    }
}
