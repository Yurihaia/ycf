use std::str::Chars;

use unicode_ident::{is_xid_continue, is_xid_start};

#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    chars: Chars<'a>,
    src: &'a str,
}

impl<'a> Cursor<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars(),
            src,
        }
    }

    #[inline]
    fn eat(&mut self) -> Option<char> {
        self.chars.next()
    }

    #[inline]
    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    #[inline]
    pub fn is_eof(&self) -> bool {
        self.peek().is_none()
    }

    #[inline]
    pub fn src(&self) -> &'a str {
        self.src
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.chars.as_str().as_ptr() as usize - self.src.as_ptr() as usize
    }

    #[inline]
    pub fn token_src(&self, token: Token) -> &'a str {
        &self.src[token.start..token.end]
    }

    #[inline]
    fn eat_while(&mut self, mut pred: impl FnMut(char) -> bool) {
        while self.peek().map(&mut pred).unwrap_or(false) {
            self.eat();
        }
    }
}

macro_rules! patterns {
    (whitespace) => {
        // same as in rust
        '\u{0009}' | // horizontal tab, '\t'
        '\u{000A}' | // line feed, '\n'
        '\u{000B}' | // vertical tab
        '\u{000C}' | // form feed
        '\u{000D}' | // carriage return, '\r'
        '\u{0020}' | // space, ' '
        '\u{0085}' | // next line
        '\u{200E}' | // left-to-right mark
        '\u{200F}' | // right-to-left mark
        '\u{2028}' | // line separator
        '\u{2029}'   // paragraph separator
    };
    (dec_digits) => {
        '0'..='9' | '_'
    };
    (hex_digits) => {
        patterns!(dec_digits) | 'a'..='f' | 'A'..='F'
    };
}

impl<'a> Cursor<'a> {
    pub fn read_token(&mut self) -> Token {
        let start = self.offset();
        // consume the next token
        let kind = self.consume_token().unwrap_or(TokenKind::Eof);
        // get the end of the token
        let end = self.offset();
        Token { kind, start, end }
    }

    fn consume_token(&mut self) -> Option<TokenKind> {
        match self.eat()? {
            '/' => match self.peek() {
                Some('/') => {
                    // eat everything until the end of the line
                    self.eat_while(|c| c != '\n');
                    // eat the newline
                    self.eat();
                    Some(TokenKind::Comment)
                }
                _ => Some(TokenKind::Unknown),
            },
            patterns!(whitespace) => {
                // eat all of the whitespace
                self.eat_while(|c| matches!(c, patterns!(whitespace)));
                Some(TokenKind::Whitespace)
            }
            c if is_xid_start(c) || c == '_' => {
                let start = self.offset() - 1;
                // eat the rest of the ident
                self.eat_while(|c| is_xid_continue(c) || c == '_' || c == '-');
                let end = self.offset();
                Some(match &self.src[start..end] {
                    "true" => TokenKind::Keyword(Keyword::True),
                    "false" => TokenKind::Keyword(Keyword::False),
                    "null" => TokenKind::Keyword(Keyword::Null),
                    _ => TokenKind::Ident,
                })
            }
            '[' => Some(TokenKind::StartSquare),
            ']' => Some(TokenKind::EndSquare),
            '{' => Some(TokenKind::StartCurly),
            '}' => Some(TokenKind::EndCurly),
            '.' => Some(TokenKind::Dot),
            '=' => Some(TokenKind::Equal),
            c @ ('0'..='9' | '-') => Some(self.consume_number(c)),
            '"' => {
                let mut terminated = false;
                while let Some(v) = self.eat() {
                    match v {
                        '"' => {
                            terminated = true;
                            break;
                        }
                        '\\' if matches!(self.peek(), Some('"' | '\\')) => {
                            self.eat();
                        }
                        '\n' => {
                            break;
                        }
                        _ => (),
                    }
                }
                Some(TokenKind::String { terminated })
            }
            _ => Some(TokenKind::Unknown),
        }
    }

    fn consume_number(&mut self, mut first: char) -> TokenKind {
        let hex_digits = |c| matches!(c, patterns!(hex_digits));
        let dec_digits = |c| matches!(c, patterns!(dec_digits));

        let sign = if first == '-' {
            first = match self.eat() {
                Some(v) => v,
                None => {
                    return TokenKind::Integer {
                        sign: true,
                        base: Base::Dec,
                    }
                }
            };
            true
        } else {
            false
        };

        if first == '0' {
            match self.peek() {
                Some('x') => {
                    self.eat();
                    // eat hex number
                    self.eat_while(hex_digits);
                    return TokenKind::Integer {
                        sign,
                        base: Base::Hex,
                    };
                }
                Some('o') => {
                    self.eat();
                    // eat oct number
                    self.eat_while(dec_digits);
                    return TokenKind::Integer {
                        sign,
                        base: Base::Oct,
                    };
                }
                Some('b') => {
                    self.eat();
                    // eat bin number
                    self.eat_while(dec_digits);
                    return TokenKind::Integer {
                        sign,
                        base: Base::Bin,
                    };
                }
                Some(patterns!(dec_digits)) => {
                    self.eat_while(dec_digits);
                }
                _ => (),
            }
        } else {
            self.eat_while(dec_digits);
        }

        match self.peek() {
            Some('.') => {
                self.eat();
                if matches!(self.peek(), Some('0'..='9')) {
                    self.eat_while(dec_digits);
                    if matches!(self.peek(), Some('e' | 'E')) {
                        if matches!(self.peek(), Some('+' | '-')) {
                            self.eat();
                        }
                        self.eat_while(dec_digits);
                    }
                }
                TokenKind::Float
            }
            Some('e' | 'E') => {
                self.eat();
                if matches!(self.peek(), Some('+' | '-')) {
                    self.eat();
                }
                self.eat_while(dec_digits);
                TokenKind::Float
            }
            _ => TokenKind::Integer {
                sign,
                base: Base::Dec,
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenKind {
    Comment,
    Whitespace,
    Ident,
    Keyword(Keyword),
    StartSquare,
    EndSquare,
    StartCurly,
    EndCurly,
    Dot,
    Equal,
    Integer { sign: bool, base: Base },
    Float,
    String { terminated: bool },
    Unknown,
    Eof,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Keyword {
    True,
    False,
    Null,
}

impl Keyword {
    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::True => "true",
            Self::False => "false",
            Self::Null => "null",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Base {
    // nothing
    Dec,
    // 0x
    Hex,
    // 0b
    Bin,
    // 0o
    Oct,
}

impl Base {
    #[inline]
    pub fn digit_offset(&self) -> usize {
        match self {
            Self::Dec => 0,
            _ => 2,
        }
    }

    #[inline]
    pub fn radix(&self) -> u32 {
        match self {
            Self::Hex => 16,
            Self::Dec => 10,
            Self::Oct => 8,
            Self::Bin => 2,
        }
    }
}