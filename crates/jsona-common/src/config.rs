use std::path::{Path, PathBuf};

use anyhow::Context;
use jsona::formatter;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::environment::Environment;
use crate::util::GlobRule;

pub const CONFIG_FILE_NAMES: &[&str] = &[".jsonarc"];

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
    pub schemas: Vec<SchemaOptions>,
    #[serde(skip)]
    pub file_rule: Option<GlobRule>,
}

impl Config {
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
            schema_opts.prepare(e, base).context("invalid schema")?;
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

    pub fn schema_for<'r>(&'r self, path: &'r Path) -> Option<&'r SchemaOptions> {
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
        if let Some(included) = &mut self.include {
            for pat in included {
                if !e.is_absolute(Path::new(pat)) {
                    *pat = base.join(pat.as_str()).to_string_lossy().into_owned();
                }
            }
        }

        if let Some(excluded) = &mut self.exclude {
            for pat in excluded {
                if !e.is_absolute(Path::new(pat)) {
                    *pat = base.join(pat.as_str()).to_string_lossy().into_owned();
                }
            }
        }

        for schema_opts in &mut self.schemas {
            if let Some(included) = &mut schema_opts.include {
                for pat in included {
                    if !e.is_absolute(Path::new(pat)) {
                        *pat = base.join(pat.as_str()).to_string_lossy().into_owned();
                    }
                }
            }

            if let Some(excluded) = &mut schema_opts.exclude {
                for pat in excluded {
                    if !e.is_absolute(Path::new(pat)) {
                        *pat = base.join(pat.as_str()).to_string_lossy().into_owned();
                    }
                }
            }
        }
    }
}

/// Options for schema validation and completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaOptions {
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

impl SchemaOptions {
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
                let p = if e.is_absolute(Path::new(&p)) {
                    PathBuf::from(p)
                } else {
                    base.join(p)
                };

                let s = p.to_string_lossy();

                Some(Url::parse(&format!("file://{s}")).context("invalid schema path")?)
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
