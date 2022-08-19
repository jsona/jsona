use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;
use url::Url;

use super::path as path_utils;

const FILE_PROTOCOL: &str = "file://";
static RE_URI_PROTOCOL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([A-Za-z_-]+://)").unwrap());

/// Judge string is a url
pub fn is_url(path: &str) -> bool {
    RE_URI_PROTOCOL
        .captures(path)
        .and_then(|v| v.get(0))
        .is_some()
}

/// Convert path to uri
pub fn to_url(path: &str, base: &Option<Url>) -> Option<Url> {
    if is_url(path) {
        return path.parse().ok();
    }
    let url = if path_utils::is_window(path) {
        format!("{}/{}", FILE_PROTOCOL, path_utils::encode_url(path))
    } else if path_utils::is_absolute(path) {
        if let Some(base) = base.as_ref() {
            let mut base: Url = base.clone();
            base.set_path("");
            format!("{}{}", base, path_utils::to_unix(path))
        } else {
            format!("{}{}", FILE_PROTOCOL, path_utils::encode_url(path))
        }
    } else {
        let base = base.as_ref()?.as_str();
        let slash = if base.ends_with('/') { "" } else { "/" };
        format!("{}{}{}", base, slash, path_utils::to_unix(path))
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
    macro_rules! assert_to_url {
        ($p:expr, $o:expr) => {
            assert_eq!(to_url($p, &None).unwrap().to_string(), $o);
        };
        ($p:expr, $b:expr, $o:expr) => {
            let b = $b.parse().ok();
            assert_eq!(to_url($p, &b).unwrap().to_string(), $o);
        };
    }
    macro_rules! assert_to_file_path {
        ($i:expr, $o:expr) => {
            assert_eq!(to_file_path(&$i.parse().unwrap()).unwrap(), $o.to_string());
        };
    }

    #[test]
    fn test_to_url() {
        assert_to_url!("a\\b", "file:///c%3A/dir1", "file:///c%3A/dir1/a/b");
        assert_to_url!("D:\\c", "file:///c%3A/dir1", "file:///d%3A/c");
        assert_to_url!("a/b", "file:///dir1", "file:///dir1/a/b");
        assert_to_url!("/a/b", "file:///dir1", "file:///a/b");
        assert_to_url!("/a/b", "file:///dir1/", "file:///a/b");
        assert_to_url!(
            "/a/b",
            "vscode-test-web://mount",
            "vscode-test-web://mount/a/b"
        );
        assert_to_url!(
            "/a/b",
            "vscode-test-web://mount/",
            "vscode-test-web://mount/a/b"
        );
        assert_to_url!(
            "a/b",
            "vscode-test-web://mount",
            "vscode-test-web://mount/a/b"
        );
        assert_to_url!(
            "a/b",
            "vscode-test-web://mount/",
            "vscode-test-web://mount/a/b"
        );
        assert_to_url!("a/b", "vscode-vfs:///dir1", "vscode-vfs:///dir1/a/b");
        assert_to_url!("http://example.com/a/b", "http://example.com/a/b");
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
        assert!(!is_url("/a/b"));
        assert!(!is_url("a/b"));
        assert!(!is_url("C:\\a\\b"));
    }
}
