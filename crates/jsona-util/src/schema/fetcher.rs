use arc_swap::ArcSwap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use url::Url;

use crate::environment::Environment;

#[derive(Clone)]
pub struct Fetcher<E: Environment> {
    env: E,
    cache_path: Arc<ArcSwap<Option<Url>>>,
    concurrent_requests: Arc<Semaphore>,
}

impl<E: Environment> Fetcher<E> {
    pub fn new(env: E) -> Self {
        Self {
            env,
            cache_path: Default::default(),
            concurrent_requests: Arc::new(Semaphore::new(10)),
        }
    }

    pub fn set_cache_path(&self, path: Option<Url>) {
        self.cache_path.swap(Arc::new(path));
    }

    #[tracing::instrument(skip_all, fields(%url))]
    pub async fn fetch(&self, url: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let data: Vec<u8> = match url.scheme() {
            "http" | "https" => self.fetch_file(url).await?,
            _ => self.env.read_file(url).await?,
        };
        Ok(data)
    }

    async fn fetch_file(&self, url: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let _permit = self.concurrent_requests.acquire().await?;
        if let Some(cache_root) = &**self.cache_path.load() {
            let cache_name = format!("{:x}", md5::compute(url.to_string().as_bytes()));
            let cache_path = cache_root.join(&cache_name)?;
            if let Ok(data) = self.env.read_file(&cache_path).await {
                tracing::debug!("fetch file from cache {}", cache_path);
                return Ok(data);
            }
            if let Ok(data) = self.env.fetch_file(url).await {
                tracing::debug!("fetch file from remote");
                if let Err(err) = self.env.write_file(&cache_path, &data).await {
                    tracing::warn!("failed to cache file {}, {}", cache_path, err);
                }
                return Ok(data);
            }
        }
        self.env.fetch_file(url).await
    }
}
