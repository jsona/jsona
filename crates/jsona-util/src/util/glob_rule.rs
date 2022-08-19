use globset::{Glob, GlobSetBuilder};
use std::{fmt::Debug, path::Path};
use url::Url;

use super::path as path_utils;
use super::url as url_utils;

#[derive(Clone)]
pub struct GlobRule {
    raw_include: Vec<String>,
    raw_exclude: Vec<String>,
    include: globset::GlobSet,
    exclude: globset::GlobSet,
}

impl Debug for GlobRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobRule")
            .field("include", &self.raw_include)
            .field("exclude", &self.raw_exclude)
            .finish()
    }
}

impl GlobRule {
    pub fn new(
        include: impl IntoIterator<Item = impl AsRef<str>>,
        exclude: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, anyhow::Error> {
        let mut inc = GlobSetBuilder::new();
        let mut raw_include = vec![];
        for glob in include {
            inc.add(Glob::new(glob.as_ref())?);
            raw_include.push(glob.as_ref().to_string());
        }

        let mut exc = GlobSetBuilder::new();
        let mut raw_exclude = vec![];
        for glob in exclude {
            exc.add(Glob::new(glob.as_ref())?);
            raw_exclude.push(glob.as_ref().to_string());
        }

        Ok(Self {
            include: inc.build()?,
            exclude: exc.build()?,
            raw_include,
            raw_exclude,
        })
    }

    pub fn preprocessing_pattern(pattern: &str, base: &Option<Url>) -> Option<String> {
        let path = path_utils::to_unix(pattern);
        if path.starts_with('/') {
            let base = base.as_ref().and_then(url_utils::to_file_path)?;
            Some(format!("{}{}", base.trim_end_matches('/'), path))
        } else {
            Some(format!("**/{}", path))
        }
    }

    pub fn is_match(&self, path: impl AsRef<Path>) -> bool {
        if !self.include.is_match(path.as_ref()) {
            return false;
        }

        !self.exclude.is_match(path.as_ref())
    }
    pub fn is_match_url(&self, url: &Url) -> bool {
        if let Some(path) = url_utils::to_file_path(url).map(path_utils::to_unix) {
            self.is_match(path)
        } else {
            false
        }
    }
}
