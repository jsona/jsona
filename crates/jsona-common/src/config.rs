use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use globset::{Glob, GlobSet, GlobSetBuilder};
use jsona::formatter;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::environment::Environment;

pub const CONFIG_FILE_NAMES: &[&str] = &[".jsonarc"];

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Formatting options.
    pub formatting: Option<formatter::OptionsIncomplete>,
    /// schema validation options.
    pub schemas: HashMap<String, SchemaOptions>,
    #[serde(skip)]
    pub file_matchers: Option<HashMap<String, GlobSet>>,
}

impl Config {
    /// Prepare the configuration for further use.
    pub fn prepare(&mut self, e: &impl Environment, base: &Path) -> Result<(), anyhow::Error> {
        let mut file_matchers = HashMap::new();
        for (name, schema_opts) in &mut self.schemas {
            let err = || format!("invalid schema `{}`", name);
            schema_opts.prepare(e, base).with_context(err)?;
            let mut matcher = GlobSetBuilder::new();
            if schema_opts.formatting.is_none() && self.formatting.is_some() {
                schema_opts.formatting = self.formatting.clone();
            }
            for glob in &schema_opts.include {
                matcher.add(Glob::new(glob.as_ref())?);
            }
            file_matchers.insert(name.to_string(), matcher.build().with_context(err)?);
        }
        self.file_matchers = Some(file_matchers);
        Ok(())
    }

    #[must_use]
    pub fn is_included(&self, path: &Path) -> bool {
        self.schema_for(path).is_some()
    }

    pub fn schema_for(&self, path: &Path) -> Option<&SchemaOptions> {
        let matchers = self.file_matchers.as_ref()?;
        for (name, matcher) in matchers {
            if matcher.is_match(path) {
                return self.schemas.get(name);
            }
        }
        None
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
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaOptions {
    /// Files to include.
    ///
    /// A list of Unix-like [glob](https://en.wikipedia.org/wiki/Glob_(programming)) path patterns.
    /// Globstars (`**`) are supported.
    ///
    /// Relative paths are **not** relative to the configuration file, but rather
    /// depends on the tool using the configuration.
    ///
    /// Omitting this property includes all files, **however an empty array will include none**.
    pub include: Vec<String>,
    /// A local file path to the schema, overrides `url` if set.
    ///
    /// For URLs, please use `url` instead.
    pub path: Option<String>,

    /// A full absolute Url to the schema.
    ///
    /// The url of the schema, supported schemes are `http`, `https`, `file` and `taplo`.
    pub url: Option<Url>,

    /// Formatting options.
    pub formatting: Option<formatter::OptionsIncomplete>,
}

impl SchemaOptions {
    fn prepare(&mut self, e: &impl Environment, base: &Path) -> Result<(), anyhow::Error> {
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
}
