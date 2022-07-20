use std::fmt::Debug;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use jsona::dom::Node;
use jsona::formatter;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::environment::Environment;
use crate::util::GlobRule;

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
    /// schema validation options.
    pub schemas: Vec<SchemaRule>,
    #[serde(skip)]
    pub file_rule: Option<GlobRule>,
}

impl Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("include", &self.include)
            .field("exclude", &self.exclude)
            .field("formatting", &self.formatting)
            .field("schemas", &self.schemas)
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
    pub fn prepare(&mut self, e: &impl Environment, base: &Path) -> Result<(), anyhow::Error> {
        self.make_absolute(e, base);

        let default_include = String::from("**/*.jsona");

        self.file_rule = Some(GlobRule::new(
            self.include
                .as_deref()
                .unwrap_or(&[default_include] as &[String]),
            self.exclude.as_deref().unwrap_or(&[] as &[String]),
        )?);

        for schema_opts in &mut self.schemas {
            schema_opts.prepare(e, base)?;
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
        self.schemas.iter().find(|v| v.is_included(path))
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

    /// Transform all relative glob patterns to have the given base path.
    fn make_absolute(&mut self, e: &impl Environment, base: &Path) {
        make_absolute_paths(&mut self.include, e, base);
        make_absolute_paths(&mut self.exclude, e, base);
        for schema_rule in &mut self.schemas {
            make_absolute_paths(&mut schema_rule.include, e, base);
            make_absolute_paths(&mut schema_rule.exclude, e, base);
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
    fn prepare(&mut self, e: &impl Environment, base: &Path) -> Result<(), anyhow::Error> {
        let default_include = String::from("**");
        self.file_rule = Some(GlobRule::new(
            self.include
                .as_deref()
                .unwrap_or(&[default_include] as &[String]),
            self.exclude.as_deref().unwrap_or(&[] as &[String]),
        )?);
        let url = match self.path.take() {
            Some(p) => {
                let s = if e.is_absolute(Path::new(&p)) {
                    PathBuf::from(p).to_string_lossy().into_owned()
                } else {
                    safe_json_path(base, p.as_str())
                };

                Some(Url::parse(&s).with_context(|| format!("invalid schema path `{s}`"))?)
            }
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

fn make_absolute_paths(paths: &mut Option<Vec<String>>, e: &impl Environment, base: &Path) {
    if let Some(paths) = paths {
        for pat in paths {
            if !e.is_absolute(Path::new(pat)) {
                *pat = safe_json_path(base, pat.as_str())
            }
        }
    }
}

fn safe_json_path(base: &Path, pat: &str) -> String {
    let output = base.join(pat).to_string_lossy().into_owned();
    if output.starts_with("file:") {
        output
    } else {
        format!("file://{}", output)
    }
}
