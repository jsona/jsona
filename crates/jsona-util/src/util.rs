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
    if path.starts_with("file://") {
        return Url::parse(path).ok();
    }
    let path = Path::new(path);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    let path = path.display().to_string();
    let path = if let Some(path) = path.strip_prefix("file://") {
        path.to_string()
    } else if cfg!(windows) {
        format!("/{}", path.replace('\\', "/"))
    } else {
        path
    };
    let path = format!("file://{}", &urlencode::encode(&path));
    Url::parse(&path).ok()
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
