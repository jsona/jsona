use std::fmt::Debug;
use std::path::Path;

use anyhow::anyhow;
use jsona::dom::Node;
use jsona::formatter;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::util::{to_file_url, get_parent_path, GlobRule};

pub const CONFIG_FILE_NAMES: &[&str] = &[".jsona"];

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Files to include.
    ///
    /// A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns.
    /// Globstars (`**`) are supported.
    ///
    /// Relative paths are **not** relative to the configuration file, but rather
    /// depends on the tool using the configuration.
    ///
    /// Omitting this property includes all files, **however an empty array will include none**.
    pub include: Option<Vec<String>>,

    /// Files to exclude (ignore).
    ///
    /// A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns.
    /// Globstars (`**`) are supported.
    ///
    /// Relative paths are **not** relative to the configuration file, but rather
    /// depends on the tool using the configuration.
    ///
    /// This has priority over `include`.
    pub exclude: Option<Vec<String>>,

    /// Formatting options.
    pub formatting: Option<formatter::OptionsIncomplete>,

    /// Rules are used to override configurations by path.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<SchemaRule>,

    #[serde(skip)]
    pub file_rule: Option<GlobRule>,
}

impl Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("include", &self.include)
            .field("exclude", &self.exclude)
            .field("formatting", &self.formatting)
            .field("rules", &self.rules)
            .finish()
    }
}

impl Config {
    /// Load config from jsona source
    pub fn from_jsona(source: &str) -> Result<Self, anyhow::Error> {
        let node: Node = source
            .parse()
            .map_err(|err| anyhow!("failed to parse jsona, {}", err))?;
        let config = serde_json::from_value(node.to_plain_json())
            .map_err(|err| anyhow!("failed to deserialize config, {}", err))?;
        Ok(config)
    }
    /// Prepare the configuration for further use.
    pub fn prepare(&mut self, config_path: &Path) -> Result<(), anyhow::Error> {
        let default_include = String::from("**/*.jsona");
        let config_dir = get_parent_path(config_path)
            .ok_or_else(|| anyhow!("invalid config_path {}", config_path.display()))?;

        self.file_rule = Some(GlobRule::new(
            self.include
                .as_deref()
                .unwrap_or(&[default_include] as &[String]),
            self.exclude.as_deref().unwrap_or(&[] as &[String]),
        )?);

        for schema_rule in &mut self.rules {
            schema_rule.prepare(&config_dir)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn is_included(&self, path: &Path) -> bool {
        match &self.file_rule {
            Some(r) => r.is_match(path),
            None => {
                tracing::debug!("no file matches were set up");
                false
            }
        }
    }

    pub fn schema_for<'r>(&'r self, path: &'r Path) -> Option<&'r SchemaRule> {
        self.rules.iter().find(|v| v.is_included(path))
    }

    pub fn update_format_options(&self, path: &Path, options: &mut formatter::Options) {
        if let Some(opts) = &self.formatting {
            options.update(opts.clone());
        }

        if let Some(schema_opts) = self.schema_for(path) {
            if let Some(opts) = schema_opts.formatting.clone() {
                options.update(opts);
            }
        }
    }
}

/// Options for schema validation and completion.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaRule {
    /// The name of the rule.
    pub name: Option<String>,
    /// Files to include.
    ///
    /// A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns.
    /// Globstars (`**`) are supported.
    ///
    /// Relative paths are **not** relative to the configuration file, but rather
    /// depends on the tool using the configuration.
    ///
    /// Omitting this property includes all files, **however an empty array will include none**.
    pub include: Option<Vec<String>>,

    /// Files to exclude (ignore).
    ///
    /// A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns.
    /// Globstars (`**`) are supported.
    ///
    /// Relative paths are **not** relative to the configuration file, but rather
    /// depends on the tool using the configuration.
    ///
    /// This has priority over `include`.
    pub exclude: Option<Vec<String>>,

    /// A local file path to the schema, overrides `url` if set.
    ///
    /// For URLs, please use `url` instead.
    pub path: Option<String>,

    /// A full absolute Url to the schema.
    ///
    /// The url of the schema, supported schemes are `http`, `https`, `file` and `jsona`.
    pub url: Option<Url>,

    /// Formatting options.
    pub formatting: Option<formatter::OptionsIncomplete>,

    #[serde(skip)]
    pub file_rule: Option<GlobRule>,
}

impl Debug for SchemaRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaRule")
            .field("name", &self.name)
            .field("include", &self.include)
            .field("exclude", &self.exclude)
            .field("path", &self.path)
            .field("url", &self.url)
            .field("formatting", &self.formatting)
            .finish()
    }
}

impl SchemaRule {
    fn prepare(&mut self, base: &Path) -> Result<(), anyhow::Error> {
        let default_include = String::from("**");
        self.file_rule = Some(GlobRule::new(
            self.include
                .as_deref()
                .unwrap_or(&[default_include] as &[String]),
            self.exclude.as_deref().unwrap_or(&[] as &[String]),
        )?);
        let url = match self.path.take() {
            Some(p) => Some(
                to_file_url(&p, Some(base.to_path_buf()))
                    .ok_or_else(|| anyhow!("invalid schema path `{}`", p))?,
            ),
            None => self.url.take(),
        };

        self.url = url;

        Ok(())
    }

    #[must_use]
    pub fn is_included(&self, path: &Path) -> bool {
        match &self.file_rule {
            Some(r) => r.is_match(path),
            None => true,
        }
    }
}
