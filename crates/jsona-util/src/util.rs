use self::path_utils::encode_url;
use globset::{Glob, GlobSetBuilder};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};
use url::Url;

const FILE_PROTOCOL: &str = "file://";
static RE_URI_PROTOCOL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\w+://)").unwrap());

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

/// Path utils works on wasm
pub mod path_utils {
    use std::{
        borrow::Cow,
        path::{Path, PathBuf},
    };

    pub fn get_parent_path(path: &Path) -> Option<PathBuf> {
        if cfg!(target_family = "wasm") {
            let raw_path = path.display().to_string();
            if is_window(&raw_path) {
                if raw_path.len() < 3 {
                    return None;
                }
                let parts: Vec<&str> = raw_path.split('\\').collect();
                let parts: Vec<&str> = parts.iter().take(parts.len() - 1).cloned().collect();
                Some(PathBuf::from(parts.join("\\")))
            } else {
                if raw_path.len() < 2 {
                    return None;
                }
                let parts: Vec<&str> = raw_path.split('/').collect();
                let parts: Vec<&str> = parts.iter().take(parts.len() - 1).cloned().collect();
                Some(PathBuf::from(parts.join("/")))
            }
        } else {
            path.parent().map(|v| v.to_path_buf())
        }
    }

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

/// Convert path to file uri
pub fn to_file_uri(path: &str, base: &Option<PathBuf>) -> Option<Url> {
    if RE_URI_PROTOCOL
        .captures(path)
        .and_then(|v| v.get(0))
        .is_some()
    {
        return path.parse().ok();
    }
    let url = if path_utils::is_window(path) {
        format!("{}/{}", FILE_PROTOCOL, encode_url(path))
    } else {
        let full_path = if path_utils::is_absolute(path) {
            path.to_string()
        } else {
            let base = match base {
                Some(v) => v.clone(),
                None => PathBuf::from("/"),
            };
            path_utils::join_path(path, &base)
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
    macro_rules! asset_to_file_uri {
        ($o:expr, $p:expr) => {
            assert_eq!(to_file_uri($p, &None).unwrap().to_string(), $o);
        };
        ($o:expr, $p:expr, $b:expr) => {
            assert_eq!(
                to_file_uri($p, &Some(PathBuf::from($b)))
                    .unwrap()
                    .to_string(),
                $o
            );
        };
    }
    macro_rules! assert_to_file_path {
        ($i:expr, $o:expr) => {
            assert_eq!(to_file_path(&$i.parse().unwrap()).unwrap(), $o.to_string());
        };
    }

    #[test]
    fn test_to_file_uri() {
        asset_to_file_uri!("file:///c%3A/dir1/a/b", "a/b", "C:\\dir1");
        asset_to_file_uri!("file:///c%3A/dir1/a/b", "a/b", "C:\\dir1\\");
        asset_to_file_uri!("file:///c%3A/dir1/a/b", "a\\b", "C:\\dir1\\");
        asset_to_file_uri!("file:///dir1/a/b", "a/b", "/dir1");
        asset_to_file_uri!("file:///dir1/a/b", "a/b", "/dir1/");
        asset_to_file_uri!("file:///dir1/a/b", "a\\b", "/dir1/");
        asset_to_file_uri!("file:///a/b", "/a/b", "/dir1");
        asset_to_file_uri!("file:///a/b", "/a/b");
        asset_to_file_uri!("file:///a/b", "file:///a/b", "/dir1");
        asset_to_file_uri!("file:///c%3A/a/b", "c:\\a\\b", "C:\\dir1");
        asset_to_file_uri!("http://example.com/a/b", "http://example.com/a/b");
    }

    #[test]
    fn test_to_file_path() {
        assert_to_file_path!("file:///c%3A/a/b", "C:\\a\\b");
        assert_to_file_path!("file:///C%3A/a/b", "C:\\a\\b");
        assert_to_file_path!("file:///dir1/a/b", "/dir1/a/b");
    }
}
