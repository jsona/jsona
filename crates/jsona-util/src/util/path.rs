use std::{borrow::Cow, path::Path};

pub fn is_window<T: AsRef<str>>(path: T) -> bool {
    let mut chars = path.as_ref().chars();
    matches!(
        (chars.next().map(|v| v.is_ascii_alphabetic()), chars.next()),
        (Some(true), Some(':'))
    )
}

pub fn is_absolute<T: AsRef<str>>(path: T) -> bool {
    let path = path.as_ref();
    if is_window(path) {
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

pub fn to_string<T: AsRef<Path>>(path: T) -> String {
    let path = path.as_ref().display().to_string();
    path.trim_end_matches(|v| v == '/' || v == '\\').to_string()
}
