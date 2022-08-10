use globset::{Glob, GlobSetBuilder};
use std::path::Path;
use url::Url;

#[derive(Debug, Clone)]
pub struct GlobRule {
    include: globset::GlobSet,
    exclude: globset::GlobSet,
}

impl GlobRule {
    pub fn new(
        include: impl IntoIterator<Item = impl AsRef<str>>,
        exclude: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, anyhow::Error> {
        let mut inc = GlobSetBuilder::new();
        for glob in include {
            inc.add(Glob::new(glob.as_ref())?);
        }

        let mut exc = GlobSetBuilder::new();
        for glob in exclude {
            exc.add(Glob::new(glob.as_ref())?);
        }

        Ok(Self {
            include: inc.build()?,
            exclude: exc.build()?,
        })
    }

    pub fn is_match(&self, text: impl AsRef<Path>) -> bool {
        if !self.include.is_match(text.as_ref()) {
            return false;
        }

        !self.exclude.is_match(text.as_ref())
    }
}

/// Convert path to file url
pub fn to_file_url(path: &str, base: &Path) -> Option<Url> {
    if let Ok(url) = Url::parse(path) {
        return Some(url);
    }
    let path = Path::new(path);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    let path = path.display().to_string();
    let path = if path.starts_with("file://") {
        path
    } else if cfg!(windows) {
        format!("file://{}", path.replace('\\', "/"))
    } else {
        format!("file://{}", path)
    };
    Url::parse(&path).ok()
}
