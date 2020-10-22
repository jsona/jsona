use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

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
            TokenKind::Eof => write!(f, "end of file"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct Position {
    index: usize,
    line: usize,
    col: usize,
}

impl Position {
    pub fn new(index: usize, line: usize, col: usize) -> Self {
        Position { index, line, col }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn col(&self) -> usize {
        self.col
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
            pos: Position::new(0, 0, 0),
            eof: false,
        }
    }
    fn step(&mut self, ch: char) {
        if ch == '\n' {
            self.pos.index += 1;
            self.pos.col = 0;
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
                        TokenKind::LexError(format!("unexpected input {}", ch)),
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
        let mut result: Vec<char> = Vec::new();
        let mut escape: Vec<char> = Vec::new();
        loop {
            let next_char = {
                if let Some(ch) = self.next_ch() {
                    ch
                } else {
                    return Some(Token::new(
                        TokenKind::LexError("unexpected string literal".into()),
                        self.pos.clone(),
                    ));
                }
            };
            match next_char {
                '\\' if escape.is_empty() => {
                    if let Some(ch) = self.peek_ch() {
                        if ch == '\n' {
                            self.next_ch();
                            continue;
                        }
                    }
                    escape.push('\\');
                }
                't' | '\\' | '\n' | 'r' if !escape.is_empty() => {
                    escape.clear();
                    result.push(next_char);
                }
                ch @ 'x' | ch @ 'u' | ch @ 'U' if !escape.is_empty() => {
                    let mut seq = escape.clone();
                    escape.clear();
                    seq.push(ch);
                    let mut out_val: u32 = 0;
                    let len = match ch {
                        'x' => 2,
                        'u' => 4,
                        'U' => 8,
                        _ => unreachable!(),
                    };
                    for _ in 0..len {
                        let c = {
                            if let Some(c) = self.peek_ch() {
                                c
                            } else {
                                return Some(Token::new(
                                    TokenKind::LexError("unexpected escape string literal".into()),
                                    start_pos,
                                ));
                            }
                        };

                        seq.push(c);
                        self.next_ch();

                        out_val *= 16;
                        let val = {
                            if let Some(val) = c.to_digit(16) {
                                val
                            } else {
                                let seq: String = seq.iter().collect();
                                return Some(Token::new(
                                    TokenKind::LexError(format!(
                                        "unexpected escape string literal {}",
                                        seq
                                    )),
                                    start_pos,
                                ));
                            }
                        };
                        out_val += val;
                    }
                    let c = {
                        if let Some(cc) = std::char::from_u32(out_val) {
                            cc
                        } else {
                            let seq: String = seq.iter().collect();
                            return Some(Token::new(
                                TokenKind::LexError(format!(
                                    "unexpected escape string literal {}",
                                    seq
                                )),
                                start_pos,
                            ));
                        }
                    };
                    result.push(c);
                }
                ch if enclosing_char == ch && !escape.is_empty() => {
                    escape.clear();
                    result.push(ch)
                }

                ch if enclosing_char == ch && escape.is_empty() => break,

                ch if !escape.is_empty() => {
                    escape.push(ch);
                    let escape: String = escape.iter().collect();
                    return Some(Token::new(
                        TokenKind::LexError(format!("unexpected escape string literal {}", escape)),
                        start_pos,
                    ));
                }
                // Cannot have new-lines inside string literals
                '\n' => {
                    return Some(Token::new(
                        TokenKind::LexError("unterminal string literal".into()),
                        start_pos,
                    ));
                }

                // All other characters
                ch => {
                    escape.clear();
                    result.push(ch);
                }
            }
        }
        let s = result.iter().collect::<String>();
        Some(Token::new(TokenKind::StringLiteral(s), start_pos))
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
