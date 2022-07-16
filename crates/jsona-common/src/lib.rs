pub mod config;
pub mod environment;
pub mod log;
pub mod schema;
pub mod util;

pub type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

pub type AsyncMutex<T> = tokio::sync::Mutex<T>;
pub type AsyncRwLock<T> = tokio::sync::RwLock<T>;

pub type LruCache<K, V> = lru::LruCache<K, V, ahash::RandomState>;
