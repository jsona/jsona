use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::ast::Position;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub position: Position,
}

impl Token {
    #[inline]
    pub fn new(kind: TokenKind, position: Position) -> Self {
        Self { kind, position }
    }
    pub fn is_value(&self) -> bool {
        match self.kind {
            TokenKind::Identifier(..)
            | TokenKind::IntegerLiteral(..)
            | TokenKind::FloatLiteral(..)
            | TokenKind::StringLiteral(..) => true,
            _ => false,
        }
    }
    pub fn get_value(&self) -> Option<String> {
        if self.is_value() {
            match self.kind.clone() {
                TokenKind::Identifier(v) => Some(v),
                TokenKind::IntegerLiteral(i) => Some(i.to_string()),
                TokenKind::FloatLiteral(f) => Some(f.to_string()),
                TokenKind::StringLiteral(v) => Some(v),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }
    pub fn is_node(&self) -> bool {
        match self.kind {
            TokenKind::LeftBrace
            | TokenKind::LeftBracket
            | TokenKind::Identifier(..)
            | TokenKind::IntegerLiteral(..)
            | TokenKind::FloatLiteral(..)
            | TokenKind::StringLiteral(..) => true,
            _ => false,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// `@`
    At,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `=`
    Eq,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// An identifier.
    Identifier(String),
    /// A integer literal.
    IntegerLiteral(i64),
    /// A float literal.
    FloatLiteral(f64),
    /// A string literal
    StringLiteral(String),
    /// A lexer error.
    LexError(String),
    /// Eof
    Eof,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::At => write!(f, "@"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Identifier(s) => write!(f, "{}", s),
            TokenKind::IntegerLiteral(i) => write!(f, "{}", i),
            TokenKind::FloatLiteral(v) => write!(f, "{}", v),
            TokenKind::StringLiteral(s) => write!(f, "{}", s),
            TokenKind::LexError(err) => write!(f, "{}", err),
            TokenKind::Eof => write!(f, "eof"),
        }
    }
}

/// Test if the given character is a hex character.
fn is_hex_char(c: char) -> bool {
    match c {
        'a'..='f' => true,
        'A'..='F' => true,
        '0'..='9' => true,
        _ => false,
    }
}

/// Test if the given character is an octal character.
fn is_octal_char(c: char) -> bool {
    match c {
        '0'..='7' => true,
        _ => false,
    }
}

/// Test if the given character is a binary character.
fn is_binary_char(c: char) -> bool {
    match c {
        '0' | '1' => true,
        _ => false,
    }
}

pub struct Lexer<T> {
    input: T,
    buf: Option<char>,
    pos: Position,
    eof: bool,
}

impl<T: Iterator<Item = char>> Lexer<T> {
    pub fn new(input: T) -> Self {
        Self {
            input,
            buf: None,
            pos: Position::new(0, 1, 1),
            eof: false,
        }
    }
    fn step(&mut self, ch: char) {
        if ch == '\n' {
            self.pos.index += 1;
            self.pos.col = 1;
            self.pos.line += 1;
        } else {
            self.pos.index += 1;
            self.pos.col += 1;
        }
    }
    fn next_ch(&mut self) -> Option<char> {
        let ch = {
            if let Some(ch) = self.buf {
                self.buf = None;
                Some(ch)
            } else if let Some(ch) = self.input.next() {
                Some(ch)
            } else {
                None
            }
        };
        if let Some(c) = ch {
            self.step(c);
        }
        ch
    }
    fn next_chars_util<F: Fn(char, usize) -> bool>(&mut self, predicate: F) -> Vec<char> {
        let mut i = 0;
        let mut output = Vec::new();
        while let Some(ch) = self.peek_ch() {
            if predicate(ch, i) {
                break;
            }
            output.push(ch);
            i += 1;
            self.next_ch();
        }
        output
    }
    fn peek_ch(&mut self) -> Option<char> {
        if let Some(ch) = self.buf {
            Some(ch)
        } else if let Some(ch) = self.input.next() {
            self.buf = Some(ch);
            Some(ch)
        } else {
            None
        }
    }
    fn peek_ch_is(&mut self, ch: char) -> bool {
        self.peek_ch().map(|c| c == ch).unwrap_or(false)
    }
    fn scan_next_token(&mut self) -> Option<Token> {
        let mut start_pos = self.pos.clone();
        while let Some(ch) = self.next_ch() {
            match (ch, self.peek_ch().unwrap_or('\0')) {
                ('@', _) => return Some(Token::new(TokenKind::At, start_pos)),
                (',', _) => return Some(Token::new(TokenKind::Comma, start_pos)),
                (':', _) => return Some(Token::new(TokenKind::Colon, start_pos)),
                ('=', _) => return Some(Token::new(TokenKind::Eq, start_pos)),
                ('{', _) => return Some(Token::new(TokenKind::LeftBrace, start_pos)),
                ('}', _) => return Some(Token::new(TokenKind::RightBrace, start_pos)),
                ('[', _) => return Some(Token::new(TokenKind::LeftBracket, start_pos)),
                (']', _) => return Some(Token::new(TokenKind::RightBracket, start_pos)),
                ('(', _) => return Some(Token::new(TokenKind::LeftParen, start_pos)),
                (')', _) => return Some(Token::new(TokenKind::RightParen, start_pos)),
                ('-', '0'..='9') => {
                    let ch_next = self.next_ch().unwrap();
                    return self.scan_number_literal(start_pos, ch_next, true);
                }
                ('-', '.') => {
                    self.next_ch();
                    return self.scan_number_literal(start_pos, '.', true);
                }
                ('.', '0'..='9') => {
                    self.next_ch();
                    return self.scan_number_literal(start_pos, '.', false);
                }
                ('/', '*') => {
                    self.next_ch();
                    loop {
                        if let Some(ch) = self.next_ch() {
                            if ch == '*' && self.peek_ch_is('/') {
                                self.next();
                                break;
                            }
                        } else {
                            return Some(Token::new(
                                TokenKind::LexError("unterminated multiline comment".into()),
                                self.pos.clone(),
                            ));
                        }
                    }
                }
                ('/', '/') => {
                    self.next_ch();
                    while let Some(ch) = self.peek_ch() {
                        if ch == '\n' {
                            break;
                        } else {
                            self.next_ch();
                        }
                    }
                }
                ('"', _) => return self.scan_string_literal(start_pos, '"'),
                ('`', _) => return self.scan_string_literal(start_pos, '`'),
                ('\'', _) => return self.scan_string_literal(start_pos, '\''),
                ('A'..='Z', _) | ('a'..='z', _) | ('_', _) => {
                    return self.scan_identifier(start_pos, ch)
                }
                ('0'..='9', _) => return self.scan_number_literal(start_pos, ch, false),
                (ch, _) if ch.is_whitespace() || ch == '\n' => {
                    start_pos = self.pos.clone();
                }
                (ch, _) => {
                    return Some(Token::new(
                        TokenKind::LexError(format!("unexpected input '{}'", ch)),
                        self.pos.clone(),
                    ))
                }
            }
        }
        if self.eof {
            return None;
        }
        self.eof = true;
        Some(Token::new(TokenKind::Eof, start_pos))
    }
    fn scan_string_literal(&mut self, start_pos: Position, enclosing_char: char) -> Option<Token> {
        let mut buf: Vec<u16> = Vec::new();
        loop {
            let ch = self.next_ch();
            if ch.is_none() {
                return Some(Token::new(TokenKind::Eof, self.pos.clone()));
            }
            let ch = ch.unwrap();
            if ch == enclosing_char {
                break;
            }
            if ch == '\n' {
                if enclosing_char == '`' {
                    buf.push(ch as u16);
                    continue;
                } else {
                    return Some(Token::new(
                        TokenKind::LexError("Unexpected line break".into()),
                        self.pos.clone(),
                    ));
                }
            }
            if ch != '\\' {
                buf.push(ch as u16);
                continue;
            }

            let next_ch = match self.next_ch() {
                Some(ch) => ch,
                None => return Some(Token::new(TokenKind::Eof, self.pos.clone())),
            };

            match next_ch {
                'b' => buf.push(8),
                'f' => buf.push(12),
                'n' => buf.push('\n' as u16),
                'r' => buf.push('\r' as u16),
                't' => buf.push(9),
                'v' => buf.push(11),
                '\'' => buf.push('\'' as u16),
                '\"' => buf.push('\"' as u16),
                'x' => {
                    // \xXX (where XX is 2 hex digits; range of 0x00–0xFF)
                    let chars = self.next_chars_util(|c, i| i > 1 || !is_hex_char(c));
                    if chars.len() != 2 {
                        return Some(Token::new(
                            TokenKind::LexError("invalid hexadecimal escape sequence".into()),
                            self.pos.clone(),
                        ));
                    }
                    buf.push(
                        u16::from_str_radix(chars.iter().collect::<String>().as_str(), 16).unwrap(),
                    )
                }
                'u' => {
                    match self.peek_ch() {
                        // Support \u{X..X} (Unicode Codepoint)
                        Some('{') => {
                            self.next_ch();
                            let chars = self.next_chars_util(|c, _| c == '}');
                            if let Some('}') = self.next_ch() {
                                let code_point = match u32::from_str_radix(
                                    &chars.iter().collect::<String>().as_str(),
                                    16,
                                ) {
                                    Err(_) => {
                                        return Some(Token::new(
                                            TokenKind::LexError(
                                                "malformed Unicode character escape sequence"
                                                    .into(),
                                            ),
                                            self.pos.clone(),
                                        ));
                                    }
                                    Ok(v) => v,
                                };

                                // UTF16Encoding of a numeric code point value
                                if code_point > 0x10_FFFF {
                                    return Some(Token::new(TokenKind::LexError("Unicode codepoint must not be greater than 0x10FFFF in escape sequence".into()), self.pos.clone()));
                                } else if code_point <= 65535 {
                                    buf.push(code_point as u16);
                                } else {
                                    let cu1 = ((code_point - 65536) / 1024 + 0xD800) as u16;
                                    let cu2 = ((code_point - 65536) % 1024 + 0xDC00) as u16;
                                    buf.push(cu1);
                                    buf.push(cu2);
                                }
                            } else {
                                return Some(Token::new(
                                    TokenKind::LexError("invalid Unicode escape sequence".into()),
                                    self.pos.clone(),
                                ));
                            }
                        }
                        Some(_) => {
                            // Collect each character after \u e.g \uD83D will give "D83D"
                            let chars = self.next_chars_util(|_, i| i > 3);
                            // Convert to u16
                            let code_point = match u16::from_str_radix(
                                &chars.iter().collect::<String>().as_str(),
                                16,
                            ) {
                                Err(_) => {
                                    return Some(Token::new(
                                        TokenKind::LexError(
                                            "malformed Unicode character escape sequence".into(),
                                        ),
                                        self.pos.clone(),
                                    ));
                                }
                                Ok(v) => v,
                            };
                            buf.push(code_point);
                        }
                        None => {
                            return Some(Token::new(
                                TokenKind::LexError("Unexpected line break".into()),
                                self.pos.clone(),
                            ))
                        }
                    }
                }
                _ => {
                    if is_octal_char(next_ch) {
                        // \XXX (where XXX is 1–3 octal digits; range of 0–377)
                        let mut chars = self.next_chars_util(|c, i| i > 1 || !is_octal_char(c));
                        chars.insert(0, next_ch);
                        buf.push(
                            u16::from_str_radix(&chars.iter().collect::<String>().as_str(), 8)
                                .unwrap(),
                        );
                    } else {
                        if next_ch.len_utf16() == 1 {
                            buf.push(next_ch as u16);
                        } else {
                            let mut code_point_bytes_buf = [0u16; 2];
                            let code_point_bytes = next_ch.encode_utf16(&mut code_point_bytes_buf);

                            buf.extend(code_point_bytes.iter());
                        }
                    }
                }
            }
        }
        Some(Token::new(
            TokenKind::StringLiteral(String::from_utf16_lossy(buf.as_slice())),
            start_pos,
        ))
    }
    fn scan_number_literal(
        &mut self,
        start_pos: Position,
        first_char: char,
        minus: bool,
    ) -> Option<Token> {
        let mut result: Vec<char> = Vec::new();
        let mut radix_base: Option<u32> = None;
        if minus {
            result.push('-');
        }
        result.push(first_char);

        while let Some(next_char) = self.peek_ch() {
            match next_char {
                '0'..='9' => {
                    result.push(next_char);
                    self.next_ch();
                }
                '.' => {
                    self.next_ch();
                    result.push(next_char);

                    while let Some(ch) = self.peek_ch() {
                        match ch {
                            '0'..='9' => {
                                result.push(ch);
                                self.next_ch();
                            }
                            _ => break,
                        }
                    }
                }
                // 0x????, 0o????, 0b????
                ch @ 'x' | ch @ 'X' | ch @ 'o' | ch @ 'O' | ch @ 'b' | ch @ 'B'
                    if first_char == '0' =>
                {
                    result.push(next_char);
                    self.next_ch();

                    let valid = match ch {
                        'x' | 'X' => is_hex_char,
                        'o' | 'O' => is_octal_char,
                        'b' | 'B' => is_binary_char,
                        _ => unreachable!(),
                    };

                    radix_base = Some(match ch {
                        'x' | 'X' => 16,
                        'o' | 'O' => 8,
                        'b' | 'B' => 2,
                        _ => unreachable!(),
                    });

                    while let Some(next_char_in_escape_seq) = self.peek_ch() {
                        if !valid(next_char_in_escape_seq) {
                            break;
                        }

                        result.push(next_char_in_escape_seq);
                        self.next_ch();
                    }
                }

                _ => break,
            }
        }

        // Parse number
        if let Some(radix) = radix_base {
            let out: String = result.iter().skip(2).collect();

            let i = {
                if let Ok(i) = i64::from_str_radix(&out, radix) {
                    i
                } else {
                    return Some(Token::new(
                        TokenKind::LexError(format!("unexpected number literal {}", out)),
                        start_pos,
                    ));
                }
            };
            Some(Token::new(TokenKind::IntegerLiteral(i), start_pos))
        } else {
            let out: String = result.iter().collect();
            if let Ok(i) = i64::from_str(&out) {
                Some(Token::new(TokenKind::IntegerLiteral(i), start_pos))
            } else if let Ok(f) = f64::from_str(&out) {
                Some(Token::new(TokenKind::FloatLiteral(f), start_pos))
            } else {
                Some(Token::new(
                    TokenKind::LexError(format!("unexpected number literal {}", out)),
                    start_pos,
                ))
            }
        }
    }
    fn scan_identifier(&mut self, start_pos: Position, first_char: char) -> Option<Token> {
        let mut result: Vec<char> = Vec::new();
        result.push(first_char);

        while let Some(next_char) = self.peek_ch() {
            if next_char.is_alphabetic() || next_char.is_digit(10) || next_char == '_' {
                result.push(next_char);
                self.next_ch();
            } else {
                break;
            }
        }

        let identifier = result.into_iter().collect();

        return Some(Token::new(TokenKind::Identifier(identifier), start_pos));
    }
}

impl<T: Iterator<Item = char>> Iterator for Lexer<T> {
    type Item = Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.scan_next_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_escape() {
        let input = "\\0\\b\\f\\n\\r\\t\\u000b\\'\\\\\\xA9\\u00A9\\u{2F804}\"";
        let mut lexer = Lexer::new(input.chars());
        let token = lexer.scan_string_literal(Position::default(), '"');
        assert_eq!(
            token,
            Some(Token::new(
                TokenKind::StringLiteral("\u{0}\u{8}\u{c}\n\r\t\u{b}\'\\©©你".into()),
                Position {
                    index: 0,
                    line: 1,
                    col: 1
                }
            ))
        );
    }
}
