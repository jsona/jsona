use rowan::TextRange;
use std::iter::{repeat, FromIterator};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

macro_rules! create_options {
    (
        $(#[$attr:meta])*
        pub struct Options {
            $(
                $(#[$field_attr:meta])*
                pub $name:ident: $ty:ty,
            )+
        }
    ) => {
        $(#[$attr])*
        pub struct Options {
            $(
                $(#[$field_attr])*
                pub $name: $ty,
            )+
        }

        impl Options {
            pub fn update(&mut self, incomplete: OptionsIncomplete) {
                $(
                    if let Some(v) = incomplete.$name {
                        self.$name = v;
                    }
                )+
            }

            pub fn update_camel(&mut self, incomplete: OptionsIncompleteCamel) {
                $(
                    if let Some(v) = incomplete.$name {
                        self.$name = v;
                    }
                )+
            }

            pub fn update_from_str<S: AsRef<str>, I: Iterator<Item = (S, S)>>(
                &mut self,
                values: I,
            ) -> Result<(), OptionParseError> {
                for (key, val) in values {

                    $(
                        if key.as_ref() == stringify!($name) {
                            self.$name =
                                val.as_ref()
                                    .parse()
                                    .map_err(|error| OptionParseError::InvalidValue {
                                        key: key.as_ref().into(),
                                        error: Box::new(error),
                                    })?;

                            continue;
                        }
                    )+

                    return Err(OptionParseError::InvalidOption(key.as_ref().into()));
                }

                Ok(())
            }
        }

        $(#[$attr])*
        #[derive(Default)]
        pub struct OptionsIncomplete {
            $(
                $(#[$field_attr])*
                pub $name: Option<$ty>,
            )+
        }

        impl OptionsIncomplete {
            pub fn from_options(opts: Options) -> Self {
                let mut o = Self::default();

                $(
                    o.$name = Some(opts.$name);
                )+

                o
            }
        }

        $(#[$attr])*
        #[derive(Default)]
        #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
        pub struct OptionsIncompleteCamel {
            $(
                $(#[$field_attr])*
                pub $name: Option<$ty>,
            )+
        }

        impl OptionsIncompleteCamel {
            pub fn from_options(opts: Options) -> Self {
                let mut o = Self::default();

                $(
                    o.$name = Some(opts.$name);
                )+

                o
            }
        }
    };
}

#[derive(Debug, Clone, Default)]
/// Scoped formatter options based on text ranges.
pub(crate) struct ScopedOptions(Vec<(TextRange, OptionsIncomplete)>);

impl ScopedOptions {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &(TextRange, OptionsIncomplete)> {
        self.0.iter()
    }
}

impl FromIterator<(TextRange, OptionsIncomplete)> for ScopedOptions {
    fn from_iter<T: IntoIterator<Item = (TextRange, OptionsIncomplete)>>(iter: T) -> Self {
        Self(Vec::from_iter(iter.into_iter()))
    }
}

create_options!(
    /// All the formatting options.
    #[derive(Debug, Clone, Eq, PartialEq)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct Options {
        /// Put trailing commas for multiline arrays/objects
        pub trailing_comma: bool,

        /// Omit whitespace padding inside single-line arrays/objects/entries.
        pub compact_mode: bool,

        /// Automatically expand arrays/objects to multiple lines
        /// if they're too long.
        pub auto_expand: bool,

        /// Automatically collapse arrays/objects if they
        /// fit in one line.
        ///
        /// The array won't be collapsed if it
        /// contains a comment or annotation.
        pub auto_collapse: bool,

        /// Target maximum column width after which
        /// arrays are expanded into new lines.
        ///
        /// This is best-effort and might not be accurate.
        pub column_width: usize,

        /// Indentation to use, should be tabs or spaces
        /// but technically could be anything.
        pub indent_string: String,

        /// Add trailing newline to the source.
        pub trailing_newline: bool,

        /// The maximum amount of consecutive blank lines allowed.
        pub allowed_blank_lines: usize,

        /// Use CRLF line endings
        pub crlf: bool,
    }
);

#[derive(Debug)]
pub enum OptionParseError {
    InvalidOption(String),
    InvalidValue {
        key: String,
        error: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl core::fmt::Display for OptionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid formatting option: {}",
            match self {
                OptionParseError::InvalidOption(k) => {
                    format!(r#"invalid option "{}""#, k)
                }
                OptionParseError::InvalidValue { key, error } => {
                    format!(r#"invalid value for option "{}": {}"#, key, error)
                }
            }
        )
    }
}

impl std::error::Error for OptionParseError {}

impl Default for Options {
    fn default() -> Self {
        Options {
            trailing_comma: true,
            compact_mode: false,
            auto_collapse: true,
            auto_expand: true,
            column_width: 80,
            indent_string: "  ".into(),
            trailing_newline: true,
            allowed_blank_lines: 2,
            crlf: false,
        }
    }
}

impl Options {
    pub(crate) fn newline(&self) -> &'static str {
        if self.crlf {
            "\r\n"
        } else {
            "\n"
        }
    }

    pub(crate) fn newlines(&self, count: usize) -> impl Iterator<Item = &'static str> {
        repeat(self.newline()).take(usize::min(count, self.allowed_blank_lines + 1))
    }
}
