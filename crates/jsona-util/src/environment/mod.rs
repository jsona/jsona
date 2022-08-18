use async_trait::async_trait;
use futures::Future;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite};
use url::Url;

#[cfg(not(target_family = "wasm"))]
pub mod native;

/// An environment in which the operations with Jsona are executed.
///
/// This is mostly needed for sandboxed environments such as WebAssembly.
#[async_trait(?Send)]
pub trait Environment: Clone + Send + Sync + 'static {
    type Stdin: AsyncRead + Unpin;
    type Stdout: AsyncWrite + Unpin;
    type Stderr: AsyncWrite + Unpin;

    fn now(&self) -> OffsetDateTime;

    fn spawn<F>(&self, fut: F)
    where
        F: Future + Send + 'static,
        F::Output: Send;

    fn spawn_local<F>(&self, fut: F)
    where
        F: Future + 'static;

    fn env_var(&self, name: &str) -> Option<String>;

    fn atty_stderr(&self) -> bool;
    fn stdin(&self) -> Self::Stdin;
    fn stdout(&self) -> Self::Stdout;
    fn stderr(&self) -> Self::Stderr;

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, anyhow::Error>;

    async fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), anyhow::Error>;

    async fn fetch_file(&self, url: &Url) -> Result<Vec<u8>, anyhow::Error>;

    /// Absolute current working dir.
    fn cwd(&self) -> Option<PathBuf>;
}
