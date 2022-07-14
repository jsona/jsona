#[derive(Debug, PartialEq, Eq)]
pub enum QuoteType {
    None,
    Double,
    Single,
    Backtick,
}

impl QuoteType {
    pub fn quote(self, no_backtick: bool) -> Self {
        if self == QuoteType::None || (no_backtick && self == QuoteType::Backtick) {
            QuoteType::Double
        } else {
            self
        }
    }
    pub fn ident(self) -> Self {
        if self == QuoteType::Backtick {
            QuoteType::Double
        } else {
            self
        }
    }
}

pub fn check_quote(s: &str) -> QuoteType {
    let mut plain = true;
    let mut backtick = false;
    let mut single = false;
    for c in s.chars() {
        if !c.is_ascii_alphanumeric() {
            plain = false;
        }
        match c {
            '\n' | '\r' => {
                backtick = true;
            }
            '"' => {
                single = true;
            }
            '\'' => {
                single = false;
            }
            _ => {}
        }
    }
    if plain {
        return QuoteType::None;
    }
    if backtick {
        return QuoteType::Backtick;
    }
    if single {
        return QuoteType::Single;
    }

    QuoteType::Double
}

pub fn quote(s: &str, quote_type: QuoteType) -> String {
    let mut output = String::new();
    let add_quote = |output: &mut String| match quote_type {
        QuoteType::None => {}
        QuoteType::Double => {
            output.push('"');
        }
        QuoteType::Single => {
            output.push('\'');
        }
        QuoteType::Backtick => {
            output.push('`');
        }
    };
    add_quote(&mut output);
    for c in s.chars() {
        match c {
            '\0' => {
                output.push_str("\\0");
            }
            '\u{8}' => {
                output.push_str("\\b");
            }
            '\u{b}' => {
                output.push_str("\\b");
            }
            '\u{c}' => {
                output.push_str("\\f");
            }
            '\\' => {
                output.push_str("\\\\");
            }
            '\t' => {
                output.push_str("\\t");
            }
            '\n' => {
                if quote_type == QuoteType::Backtick {
                    output.push(c);
                } else {
                    output.push_str("\\n");
                }
            }
            '\r' => {
                if quote_type == QuoteType::Backtick {
                    output.push(c);
                } else {
                    output.push_str("\\r");
                }
            }
            '\'' => {
                if quote_type == QuoteType::Single {
                    output.push_str("\\'");
                } else {
                    output.push(c);
                }
            }
            '"' => {
                if quote_type == QuoteType::Double {
                    output.push_str("\\\"");
                } else {
                    output.push(c);
                }
            }
            '`' => {
                if quote_type == QuoteType::Backtick {
                    output.push_str("\\`");
                } else {
                    output.push(c);
                }
            }
            _ => {
                output.push(c);
            }
        }
    }
    add_quote(&mut output);
    output
}
