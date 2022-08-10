use figment::{providers::Serialized, Figment};
use jsona_util::{
    schema::{associations::DEFAULT_SCHEMASTORES, cache::DEFAULT_LRU_CACHE_EXPIRATION_TIME},
    HashMap,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitConfig {
    pub cache_path: Option<PathBuf>,
    #[serde(default = "default_configuration_section")]
    pub configuration_section: String,
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            cache_path: Default::default(),
            configuration_section: default_configuration_section(),
        }
    }
}

fn default_configuration_section() -> String {
    String::from("jsona")
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspConfig {
    pub config_file: ConfigFileConfig,
    pub schema: SchemaConfig,
    pub formatter: jsona::formatter::OptionsIncompleteCamel,
}

impl LspConfig {
    pub fn update_from_json(&mut self, json: &Value) -> Result<(), anyhow::Error> {
        *self = Figment::new()
            .merge(Serialized::defaults(&self))
            .merge(Serialized::defaults(json))
            .extract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaConfig {
    pub enabled: bool,
    pub associations: HashMap<String, String>,
    pub stores: Vec<Url>,
    pub links: bool,
    pub cache: SchemaCacheConfig,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            associations: Default::default(),
            stores: DEFAULT_SCHEMASTORES
                .iter()
                .map(|c| c.parse().unwrap())
                .collect(),
            links: false,
            cache: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaCacheConfig {
    pub memory_expiration: u64,
    pub disk_expiration: u64,
}

impl Default for SchemaCacheConfig {
    fn default() -> Self {
        Self {
            memory_expiration: DEFAULT_LRU_CACHE_EXPIRATION_TIME.as_secs(),
            disk_expiration: DEFAULT_LRU_CACHE_EXPIRATION_TIME.as_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileConfig {
    pub path: Option<PathBuf>,
    pub enabled: bool,
}

impl Default for ConfigFileConfig {
    fn default() -> Self {
        Self {
            path: Default::default(),
            enabled: true,
        }
    }
}
