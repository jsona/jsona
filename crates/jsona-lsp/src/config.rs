use figment::{providers::Serialized, Figment};
use jsona_util::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use url::Url;

pub const DEFAULT_CONFIGURATION_SECTION: &str = "jsona";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitializationOptions {
    pub cache_path: Option<PathBuf>,
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
    pub associations: HashMap<String, Vec<String>>,
    pub cache: bool,
    pub store_url: Option<Url>,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache: true,
            associations: Default::default(),
            store_url: None,
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
