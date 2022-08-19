use async_trait::async_trait;
use futures::Future;
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite};
use url::Url;

use crate::util::url::to_url;

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

    async fn read_file(&self, path: &Url) -> Result<Vec<u8>, anyhow::Error>;

    async fn write_file(&self, path: &Url, bytes: &[u8]) -> Result<(), anyhow::Error>;

    async fn fetch_file(&self, path: &Url) -> Result<Vec<u8>, anyhow::Error>;

    fn root_uri(&self) -> Option<Url>;

    fn to_url(&self, path: &str) -> Option<Url> {
        to_url(path, &self.root_uri())
    }
}
