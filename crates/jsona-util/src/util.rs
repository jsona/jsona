use self::path_utils::{encode_url, to_unix};
use globset::{Glob, GlobSetBuilder};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{borrow::Cow, fmt::Debug, path::Path};
use url::Url;

const FILE_PROTOCOL: &str = "file://";
static RE_URI_PROTOCOL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([A-Za-z_-]+://)").unwrap());

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
        let path = to_unix(pattern);
        if path.starts_with('/') {
            let base = base.as_ref().and_then(to_file_path)?;
            Some(format!("{}{}", base, path))
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
        if let Some(path) = to_file_path(url).map(to_unix) {
            self.is_match(path)
        } else {
            false
        }
    }
}

/// Path utils works on wasm
pub mod path_utils {
    use std::{borrow::Cow, path::Path};

    pub fn join_path<T: AsRef<str>>(path: T, base: &Path) -> String {
        let path: &str = path.as_ref();
        if is_absolute(path) {
            return path.to_string();
        }
        let base = remove_tail_slash(base.display().to_string());
        if is_window(&base) {
            let (driver, base_path) = base.split_at(2);
            let path = path.replace('/', "\\");
            format!(
                "{}{}{}{}",
                driver.to_ascii_uppercase(),
                base_path,
                "\\",
                path
            )
        } else {
            let path = path.replace('\\', "/");
            format!("{}{}{}", base, "/", path)
        }
    }

    pub fn is_window<T: AsRef<str>>(path: T) -> bool {
        let mut chars = path.as_ref().chars();
        matches!(
            (chars.next().map(|v| v.is_ascii_alphabetic()), chars.next()),
            (Some(true), Some(':'))
        )
    }

    pub fn is_absolute<T: AsRef<str>>(path: T) -> bool {
        let path = path.as_ref();
        if is_window(&path) {
            true
        } else {
            path.starts_with('/')
        }
    }

    pub fn to_unix<T: AsRef<str>>(path: T) -> String {
        path.as_ref().replace('\\', "/")
    }

    pub fn encode_url<T: AsRef<str>>(path: T) -> String {
        let path = if is_window(&path) {
            let (driver, tail) = path.as_ref().split_at(1);
            format!(
                "/{}{}",
                driver.to_ascii_lowercase(),
                tail.replace('\\', "/")
            )
        } else {
            path.as_ref().to_string()
        };
        to_unix(path)
            .split('/')
            .map(urlencoding::encode)
            .collect::<Vec<Cow<str>>>()
            .join("/")
    }

    pub fn remove_tail_slash<T: AsRef<str>>(path: T) -> String {
        path.as_ref()
            .trim_end_matches(|v| v == '/' || v == '\\')
            .to_string()
    }
}

/// Judge string is a url
pub fn is_url(path: &str) -> bool {
    RE_URI_PROTOCOL
        .captures(path)
        .and_then(|v| v.get(0))
        .is_some()
}

/// Convert path to file uri
pub fn to_file_uri(path: &str, base: &Option<Url>) -> Option<Url> {
    if is_url(path) {
        return path.parse().ok();
    }
    let url = if path_utils::is_window(path) {
        format!("{}/{}", FILE_PROTOCOL, encode_url(path))
    } else {
        let full_path = if path_utils::is_absolute(path) {
            path.to_string()
        } else {
            let base = base.as_ref().and_then(to_file_path)?;
            path_utils::join_path(path, Path::new(&base))
        };
        format!("{}{}", FILE_PROTOCOL, encode_url(full_path))
    };
    url.parse().ok()
}

/// Convert url to file path
pub fn to_file_path(url: &Url) -> Option<String> {
    let path = url.path();
    let mut chars = path.chars();
    let mut driver = None;
    let check_driver = |v: char, driver: &mut Option<char>| {
        if v.is_ascii_alphabetic() {
            *driver = Some(v);
            true
        } else {
            false
        }
    };
    if matches!(
        (
            chars.next(),
            chars.next().map(|v| check_driver(v, &mut driver)),
            chars.next(),
            chars.next(),
            chars.next()
        ),
        (Some('/'), Some(true), Some('%'), Some('3'), Some('A'))
    ) {
        // windows driver `/c%3A` => `/C:`
        let (_, path) = path.split_at(5);
        let path = path
            .split('/')
            .map(|v| urlencoding::decode(v).ok())
            .collect::<Option<Vec<Cow<str>>>>()?
            .join("\\");
        Some(format!("{}:{}", driver.unwrap().to_ascii_uppercase(), path))
    } else {
        Some(path.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! assert_to_file_uri {
        ($o:expr, $p:expr) => {
            assert_eq!(to_file_uri($p, &None).unwrap().to_string(), $o);
        };
        ($o:expr, $p:expr, $b:expr) => {
            let b = $b.parse().ok();
            assert_eq!(to_file_uri($p, &b).unwrap().to_string(), $o);
        };
    }
    macro_rules! assert_to_file_path {
        ($i:expr, $o:expr) => {
            assert_eq!(to_file_path(&$i.parse().unwrap()).unwrap(), $o.to_string());
        };
    }

    #[test]
    fn test_to_file_uri() {
        assert_to_file_uri!("file:///c%3A/dir1/a/b", "a/b", "file:///c%3A/dir1");
        assert_to_file_uri!("file:///c%3A/dir1/a/b", "a\\b", "file:///c%3A/dir1");
        assert_to_file_uri!("http://example.com/a/b", "http://example.com/a/b");
    }

    #[test]
    fn test_to_file_path() {
        assert_to_file_path!("file:///c%3A/a/b", "C:\\a\\b");
        assert_to_file_path!("file:///C%3A/a/b", "C:\\a\\b");
        assert_to_file_path!("file:///dir1/a/b", "/dir1/a/b");
        assert_to_file_path!(
            "vscode-vfs://github/jsona/schemastore",
            "/jsona/schemastore"
        );
    }

    #[test]
    fn test_is_url() {
        assert!(is_url("file:///c%3A/a/b"));
        assert!(is_url("vscode-test-web://mount"));
        assert!(is_url("vscode-vfs://github/jsona/jsona"));
    }
}
