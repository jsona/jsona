use globset::{Glob, GlobSetBuilder};
use std::path::{Path, PathBuf};
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

pub const FILE_PROTOCOL: &str = "file://";

/// Convert path to file url
pub fn to_file_url(path: &str, base: Option<PathBuf>) -> Option<Url> {
    fn is_window_path(path: &str) -> bool {
        let mut chars = path.chars();
        matches!(
            (chars.next().map(|v| v.is_ascii_alphabetic()), chars.next()),
            (Some(true), Some(':'))
        )
    }
    fn fix_window_path(path: String) -> String {
        path.chars()
            .enumerate()
            .map(|(i, v)| if i == 0 { v.to_ascii_lowercase() } else { v })
            .collect()
    }
    let path = if let Some(tail) = path.strip_prefix(FILE_PROTOCOL) {
        tail
    } else {
        path
    };
    let path = path.replace('\\', "/");
    let path = if path.starts_with('/') {
        format!("{}{}", FILE_PROTOCOL, urlencode::encode(&path))
    } else if is_window_path(&path) {
        format!(
            "{}/{}",
            FILE_PROTOCOL,
            urlencode::encode(&fix_window_path(path))
        )
    } else {
        let base = match base {
            None => String::new(),
            Some(base) => base.display().to_string().replace('\\', "/"),
        };
        let full_path = if base.ends_with('/') {
            format!("{}{}", base, path)
        } else {
            format!("{}/{}", base, path)
        };
        if is_window_path(&base) {
            format!(
                "{}/{}",
                FILE_PROTOCOL,
                urlencode::encode(&fix_window_path(full_path))
            )
        } else {
            format!("{}{}", FILE_PROTOCOL, urlencode::encode(&full_path))
        }
    };
    path.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! asset_to_file_url {
        ($o:expr, $p:expr) => {
            assert_eq!($o, to_file_url($p, None).unwrap().to_string());
        };
        ($o:expr, $p:expr, $b:expr) => {
            assert_eq!(
                $o,
                to_file_url($p, Some(Path::new($b).to_path_buf()))
                    .unwrap()
                    .to_string()
            );
        };
    }

    #[test]
    fn test_to_file_url() {
        asset_to_file_url!("file:///c%3A/dir1/a/b", "a/b", "C:\\dir1");
        asset_to_file_url!("file:///c%3A/dir1/a/b", "a/b", "C:\\dir1\\");
        asset_to_file_url!("file:///c%3A/dir1/a/b", "a\\b", "C:\\dir1\\");
        asset_to_file_url!("file:///dir1/a/b", "a/b", "/dir1");
        asset_to_file_url!("file:///dir1/a/b", "a/b", "/dir1/");
        asset_to_file_url!("file:///dir1/a/b", "a\\b", "/dir1/");
        asset_to_file_url!("file:///a/b", "/a/b", "/dir1");
        asset_to_file_url!("file:///a/b", "/a/b");
        asset_to_file_url!("file:///a/b", "file:///a/b", "/dir1");
        asset_to_file_url!("file:///c%3A/a/b", "c:\\a\\b", "C:\\dir1");
    }
}

mod urlencode {
    use std::borrow::Cow;
    use std::str;

    /// Percent-encodes every byte except alphanumerics and `-`, `_`, `.`, `~`, '/'. Assumes UTF-8 encoding.
    ///
    /// Call `.into_owned()` if you need a `String`
    #[inline(always)]
    pub fn encode(data: &str) -> Cow<str> {
        encode_binary(data.as_bytes())
    }

    /// Percent-encodes every byte except alphanumerics and `-`, `_`, `.`, `~`, '/'.
    #[inline]
    pub fn encode_binary(data: &[u8]) -> Cow<str> {
        // add maybe extra capacity, but try not to exceed allocator's bucket size
        let mut escaped = String::with_capacity(data.len() | 15);
        let unmodified = append_string(data, &mut escaped, true);
        if unmodified {
            return Cow::Borrowed(unsafe {
                // encode_into has checked it's ASCII
                str::from_utf8_unchecked(data)
            });
        }
        Cow::Owned(escaped)
    }

    fn append_string(data: &[u8], escaped: &mut String, may_skip: bool) -> bool {
        encode_into(data, may_skip, |s| {
            escaped.push_str(s);
            Ok::<_, std::convert::Infallible>(())
        })
        .unwrap()
    }

    fn encode_into<E>(
        mut data: &[u8],
        may_skip_write: bool,
        mut push_str: impl FnMut(&str) -> Result<(), E>,
    ) -> Result<bool, E> {
        let mut pushed = false;
        loop {
            // Fast path to skip over safe chars at the beginning of the remaining string
            let ascii_len = data.iter()
				.take_while(|&&c| matches!(c, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' |  b'-' | b'.' | b'_' | b'~' | b'/')).count();

            let (safe, rest) = if ascii_len >= data.len() {
                if !pushed && may_skip_write {
                    return Ok(true);
                }
                (data, &[][..]) // redundatnt to optimize out a panic in split_at
            } else {
                data.split_at(ascii_len)
            };
            pushed = true;
            if !safe.is_empty() {
                push_str(unsafe { str::from_utf8_unchecked(safe) })?;
            }
            if rest.is_empty() {
                break;
            }

            match rest.split_first() {
                Some((byte, rest)) => {
                    let enc = &[b'%', to_hex_digit(byte >> 4), to_hex_digit(byte & 15)];
                    push_str(unsafe { str::from_utf8_unchecked(enc) })?;
                    data = rest;
                }
                None => break,
            };
        }
        Ok(false)
    }

    #[inline]
    fn to_hex_digit(digit: u8) -> u8 {
        match digit {
            0..=9 => b'0' + digit,
            10..=255 => b'A' - 10 + digit,
        }
    }
}
