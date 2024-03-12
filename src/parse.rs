use std::{fmt, mem, str::FromStr};

use crate::cursor::{Cursor, Keyword, Token, TokenKind};

#[derive(Clone, Debug)]
pub struct Parser<'a> {
    pub cursor: Cursor<'a>,
    // these are 0 indexed
    pub line: u32,
    pub col: u32,
    // whether to skip whitespace and comments
    pub skip: bool,
    pub lookahead: Option<SpanToken>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            cursor: Cursor::new(src),
            line: 0,
            col: 0,
            skip: true,
            lookahead: None,
        }
    }

    pub fn next_token(&mut self) -> SpanToken {
        if let Some(token) = self.lookahead.take() {
            if self.skip(token.token.kind) {
                return self.next_token();
            }
            return token;
        }

        let line = self.line;
        let col = self.col;
        let token = self.cursor.read_token();
        let src = self.cursor.token_src(token);
        // advance line/col count
        // handle \n
        if matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment) {
            // find the number of newlines and the length of the final line
            let (line, last) = src
                .split('\n')
                .enumerate()
                .last()
                .expect("split should not return None");
            self.line += line as u32;
            if line >= 1 {
                self.col = 0;
            }
            self.col += last.len() as u32;
        } else {
            self.col += src.len() as u32;
        }

        if self.skip(token.kind) {
            return self.next_token();
        }

        SpanToken { token, line, col }
    }

    pub fn peek_token(&mut self) -> SpanToken {
        // return the lookahead token if it is present
        if let Some(token) = self.lookahead {
            if !self.skip(token.token.kind) {
                return token;
            }
        }
        // otherwise get the next token
        let next = self.next_token();
        // store it in the lookahead
        self.lookahead = Some(next);
        // and return it
        next
    }

    pub fn next_no_skip(&mut self) -> SpanToken {
        let old = mem::replace(&mut self.skip, false);
        let res = self.next_token();
        self.skip = old;
        res
    }

    pub fn peek_no_skip(&mut self) -> SpanToken {
        let old = mem::replace(&mut self.skip, false);
        let res = self.peek_token();
        self.skip = old;
        res
    }

    fn skip(&self, kind: TokenKind) -> bool {
        self.skip && matches!(kind, TokenKind::Whitespace | TokenKind::Comment)
    }

    pub fn src(&self, token: Token) -> &'a str {
        self.cursor.token_src(token)
    }

    pub fn peek_eof(&mut self) -> bool {
        self.peek_token().token.kind == TokenKind::Eof
    }

    pub fn error(&mut self, kind: ParseErrorKind) -> ParseError {
        ParseError::new(self.peek_token(), kind)
    }

    pub fn parse_null(&mut self) -> Result<(), ParseError> {
        let next = self.next_token();
        match next.token.kind {
            TokenKind::Keyword(Keyword::Null) => Ok(()),
            _ => Err(ParseError::new(next, ParseErrorKind::ExpectedBool)),
        }
    }

    pub fn try_parse_null(&mut self) -> Option<Result<(), ParseError>> {
        let next = self.peek_token();
        match next.token.kind {
            TokenKind::Keyword(Keyword::Null) => {
                self.next_token();
                Some(Ok(()))
            }
            _ => None,
        }
    }

    pub fn parse_bool(&mut self) -> Result<bool, ParseError> {
        let next = self.next_token();
        match next.token.kind {
            TokenKind::Keyword(Keyword::True) => Ok(true),
            TokenKind::Keyword(Keyword::False) => Ok(false),
            _ => Err(ParseError::new(next, ParseErrorKind::ExpectedBool)),
        }
    }

    pub fn try_parse_bool(&mut self) -> Option<Result<bool, ParseError>> {
        let next = self.peek_token();
        let val = match next.token.kind {
            TokenKind::Keyword(Keyword::True) => true,
            TokenKind::Keyword(Keyword::False) => false,
            _ => return None,
        };
        // consume peeked token
        self.next_token();
        Some(Ok(val))
    }

    pub fn parse_string(&mut self) -> Result<String, ParseError> {
        let next = self.next_token();
        if let TokenKind::String { terminated } = next.token.kind {
            if !terminated {
                return Err(ParseError::new(next, ParseErrorKind::StringUnterminated));
            }
            let mut chars = self.src(next.token).chars().enumerate();
            // skip the starting '"'
            chars.next();

            let mut out = String::with_capacity(self.src(next.token).len() - 2);
            while let Some((pos, c)) = chars.next() {
                match c {
                    '\\' => {
                        let esc = chars.next().expect("string should be terminated").1;
                        let ive = || {
                            ParseError::new(next, ParseErrorKind::InvalidEscape { pos: pos as u32 })
                        };
                        match esc {
                            'n' => out.push('\n'),
                            'r' => out.push('\r'),
                            't' => out.push('\t'),
                            '0' => out.push('\0'),
                            '\\' => out.push('\\'),
                            '"' => out.push('"'),
                            'x' => {
                                let mut val = 0;
                                val |= chars
                                    .next()
                                    .and_then(|(_, c)| c.to_digit(16))
                                    .ok_or_else(ive)?;
                                if val > 0x7 {
                                    return Err(ive());
                                }
                                val <<= 4;
                                val |= chars
                                    .next()
                                    .and_then(|(_, c)| c.to_digit(16))
                                    .ok_or_else(ive)?;
                                out.push(char::from_u32(val).ok_or_else(ive)?);
                            }
                            'u' => {
                                if !matches!(chars.next(), Some((_, '{'))) {
                                    return Err(ive());
                                }
                                let mut ct = 0;
                                let mut val = 0;
                                while let Some((_, c)) = chars.next() {
                                    if c == '}' {
                                        break;
                                    } else if ct >= 6 {
                                        return Err(ive());
                                    }
                                    ct += 1;
                                    val <<= 4;
                                    val |= chars
                                        .next()
                                        .and_then(|(_, c)| c.to_digit(16))
                                        .ok_or_else(ive)?;
                                }
                                if ct == 0 {
                                    return Err(ive());
                                }
                                out.push(char::from_u32(val).ok_or_else(ive)?);
                            }
                            _ => return Err(ive()),
                        }
                    }
                    '"' => {
                        break;
                    }
                    _ => out.push(c),
                }
            }
            Ok(out)
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedString))
        }
    }

    pub fn try_parse_string(&mut self) -> Option<Result<String, ParseError>> {
        if let TokenKind::String { .. } = self.peek_token().token.kind {
            Some(self.parse_string())
        } else {
            None
        }
    }

    pub fn parse_uint<T>(&mut self) -> Result<T, ParseError>
    where
        T: TryFrom<u64>,
    {
        let next = self.next_token();
        if let TokenKind::Integer { base, sign: false } = next.token.kind {
            let start = next.token.start + base.digit_offset();
            let end = next.token.end;
            let src = &self.cursor.src()[start..end];
            let val = match u64::from_str_radix(src, base.radix()) {
                Ok(v) => v,
                Err(_) => return Err(ParseError::new(next, ParseErrorKind::InvalidInteger)),
            };

            T::try_from(val).map_err(|_| ParseError::new(next, ParseErrorKind::InvalidInteger))
        } else {
            Err(ParseError::new(
                next,
                ParseErrorKind::ExpectedInteger { signed: false },
            ))
        }
    }

    pub fn parse_int<T>(&mut self) -> Result<T, ParseError>
    where
        T: TryFrom<i64>,
    {
        let next = self.next_token();
        if let TokenKind::Integer { base, sign } = next.token.kind {
            let start = next.token.start + base.digit_offset() + sign as usize;
            let end = next.token.end;
            let src = &self.cursor.src()[start..end];

            let val = match u64::from_str_radix(src, base.radix()) {
                Ok(v) => v,
                Err(_) => return Err(ParseError::new(next, ParseErrorKind::InvalidInteger)),
            };

            let int = if sign {
                let val = val.wrapping_neg() as i64;
                // if the cast results in a (double) wrapped value
                // the source integer is an underflow
                if val > 0 {
                    return Err(ParseError::new(next, ParseErrorKind::InvalidInteger));
                } else {
                    val
                }
            } else if let Ok(v) = i64::try_from(val) {
                v
            } else {
                return Err(ParseError::new(next, ParseErrorKind::InvalidInteger));
            };

            T::try_from(int).map_err(|_| ParseError::new(next, ParseErrorKind::InvalidInteger))
        } else {
            Err(ParseError::new(
                next,
                ParseErrorKind::ExpectedInteger { signed: true },
            ))
        }
    }

    pub fn parse_float<T>(&mut self) -> Result<T, ParseError>
    where
        T: FromStr,
    {
        let next = self.next_token();
        if let TokenKind::Float = next.token.kind {
            let start = next.token.start;
            let end = next.token.end;
            let src = &self.cursor.src()[start..end];
            src.parse()
                .map_err(|_| ParseError::new(next, ParseErrorKind::InvalidFloat))
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedFloat))
        }
    }

    pub fn try_parse_uint(&mut self) -> Option<Result<u64, ParseError>> {
        if let TokenKind::Integer { sign: false, .. } = self.peek_token().token.kind {
            Some(self.parse_uint())
        } else {
            None
        }
    }

    pub fn try_parse_int(&mut self) -> Option<Result<i64, ParseError>> {
        if let TokenKind::Integer { .. } = self.peek_token().token.kind {
            Some(self.parse_int())
        } else {
            None
        }
    }

    pub fn try_parse_float(&mut self) -> Option<Result<f64, ParseError>> {
        if let TokenKind::Float = self.peek_token().token.kind {
            Some(self.parse_float())
        } else {
            None
        }
    }

    pub fn start_map(&mut self) -> Result<(), ParseError> {
        let next = self.next_token();
        if let TokenKind::StartCurly = next.token.kind {
            Ok(())
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedMapStart))
        }
    }

    pub fn map_delimiter(&mut self) -> Result<(), ParseError> {
        let next = self.next_token();
        if let TokenKind::Equal = next.token.kind {
            Ok(())
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedEqual))
        }
    }

    pub fn end_map(&mut self) -> Result<(), ParseError> {
        let next = self.next_token();
        if let TokenKind::EndCurly = next.token.kind {
            Ok(())
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedMapEnd))
        }
    }

    pub fn try_start_map(&mut self) -> Option<()> {
        if let TokenKind::StartCurly = self.peek_token().token.kind {
            self.next_token();
            Some(())
        } else {
            None
        }
    }

    pub fn try_map_delimiter(&mut self) -> Option<()> {
        if let TokenKind::Equal = self.peek_token().token.kind {
            self.next_token();
            Some(())
        } else {
            None
        }
    }

    // pub fn try_end_map(&mut self) -> Option<()> {
    //     if let TokenKind::EndCurly = self.peek_token().token.kind {
    //         self.next_token();
    //         Some(())
    //     } else {
    //         None
    //     }
    // }
    
    pub fn peek_end_map(&mut self) -> bool {
        TokenKind::EndCurly == self.peek_token().token.kind
    }

    pub fn parse_path(&mut self) -> Result<MapPath, ParseError> {
        let next = self.next_token();
        if let TokenKind::Ident = next.token.kind {
            let first = self.src(next.token).to_string();
            let mut path = Vec::new();
            while let TokenKind::Dot = self.peek_no_skip().token.kind {
                self.next_no_skip();
                let next_segment = self.next_no_skip();
                if let TokenKind::Ident = next_segment.token.kind {
                    path.push(self.src(next_segment.token).to_string());
                } else {
                    return Err(ParseError::new(next_segment, ParseErrorKind::ExpectedIdent));
                }
            }
            Ok(MapPath { key: first, path })
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedIdent))
        }
    }

    pub fn try_parse_path(&mut self) -> Option<Result<MapPath, ParseError>> {
        if let TokenKind::Ident = self.peek_token().token.kind {
            Some(self.parse_path())
        } else {
            None
        }
    }

    pub fn start_list(&mut self) -> Result<(), ParseError> {
        let next = self.next_token();
        if let TokenKind::StartSquare = next.token.kind {
            Ok(())
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedListStart))
        }
    }

    pub fn end_list(&mut self) -> Result<(), ParseError> {
        let next = self.next_token();
        if let TokenKind::EndSquare = next.token.kind {
            Ok(())
        } else {
            Err(ParseError::new(next, ParseErrorKind::ExpectedListEnd))
        }
    }

    pub fn try_start_list(&mut self) -> Option<()> {
        if let TokenKind::StartSquare = self.peek_token().token.kind {
            self.next_token();
            Some(())
        } else {
            None
        }
    }

    // pub fn try_end_list(&mut self) -> Option<()> {
    //     if let TokenKind::EndSquare = self.peek_token().token.kind {
    //         self.next_token();
    //         Some(())
    //     } else {
    //         None
    //     }
    // }
    
    pub fn peek_end_list(&mut self) -> bool {
        TokenKind::EndSquare == self.peek_token().token.kind
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPath {
    pub key: String,
    pub path: Vec<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpanToken {
    pub token: Token,
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub token: SpanToken,
    pub kind: ParseErrorKind,
}

impl ParseError {
    pub fn new(token: SpanToken, kind: ParseErrorKind) -> Self {
        Self { token, kind }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.kind.display(self.token),
            self.token.line,
            self.token.col,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    ExpectedNull,
    ExpectedBool,
    ExpectedInteger { signed: bool },
    ExpectedFloat,
    ExpectedMapStart,
    ExpectedMapEnd,
    ExpectedListStart,
    ExpectedListEnd,
    ExpectedEqual,
    InvalidInteger,
    InvalidFloat,
    ExpectedString,
    InvalidEscape { pos: u32 },
    StringUnterminated,
    ExpectedIdent,
    UnknownToken,
}

fn display_token_kind(kind: TokenKind) -> impl fmt::Display {
    struct Proxy(TokenKind);
    impl fmt::Display for Proxy {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.0 {
                TokenKind::Comment => write!(f, "a comment"),
                TokenKind::Whitespace => write!(f, "whitespace"),
                TokenKind::Ident => write!(f, "an identifier"),
                TokenKind::Keyword(v) => write!(f, "'{}'", v.as_str()),
                TokenKind::StartSquare => write!(f, "'['"),
                TokenKind::EndSquare => write!(f, "']'"),
                TokenKind::StartCurly => write!(f, "'{{'"),
                TokenKind::EndCurly => write!(f, "'}}'"),
                TokenKind::Dot => write!(f, "'.'"),
                TokenKind::Integer { sign: true, .. } => write!(f, "a signed Integer"),
                TokenKind::Integer { sign: false, .. } => write!(f, "an unsigned Integer"),
                TokenKind::Float => write!(f, "a floating point number"),
                TokenKind::String { .. } => write!(f, "a string"),
                TokenKind::Unknown => write!(f, "an unknown token"),
                TokenKind::Eof => write!(f, "the end of the file"),
                TokenKind::Equal => write!(f, "'='"),
            }
        }
    }
    Proxy(kind)
}

impl ParseErrorKind {
    fn display(&self, token: SpanToken) -> impl fmt::Display + '_ {
        struct Proxy<'s>(&'s ParseErrorKind, SpanToken);

        impl<'a> fmt::Display for Proxy<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut expected = |s: &str| {
                    write!(
                        f,
                        "Expected {}, found {}",
                        s,
                        display_token_kind(self.1.token.kind)
                    )
                };

                use ParseErrorKind::*;
                match self.0 {
                    ExpectedNull => expected("null"),
                    ExpectedBool => expected("a bool"),
                    ExpectedInteger { signed: false } => expected("an unsigned integer"),
                    ExpectedInteger { signed: true } => expected("a signed integer"),
                    ExpectedFloat => expected("a floating point number"),
                    ExpectedMapStart => expected("a map"),
                    ExpectedMapEnd => expected("'}'"),
                    ExpectedListStart => expected("a list"),
                    ExpectedListEnd => expected("']'"),
                    ExpectedEqual => expected("'='"),
                    ExpectedString => expected("a string"),
                    ExpectedIdent => expected("an identifier"),
                    InvalidInteger => write!(f, "Invalid integer"),
                    InvalidFloat => write!(f, "Invalid float"),
                    StringUnterminated => write!(f, "Expected a closing '\"'"),
                    InvalidEscape { pos } => write!(
                        f,
                        "Invalid escape sequence at character {} of string",
                        pos.saturating_sub(1)
                    ),
                    UnknownToken => {
                        write!(f, "Unknown token {}", display_token_kind(self.1.token.kind))
                    }
                }
            }
        }

        Proxy(self, token)
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Debug)]
    enum Value {
        Null,
        Bool(bool),
        U64(u64),
        I64(i64),
        F64(f64),
        String(String),
        List(Vec<Value>),
        Map(HashMap<String, Value>),
    }

    impl Value {
        fn parse(p: &mut Parser) -> Result<Self, ParseError> {
            if p.try_parse_null().is_some() {
                Ok(Self::Null)
            } else if let Some(r) = p.try_parse_bool() {
                Ok(Self::Bool(r?))
            } else if let Some(r) = p.try_parse_uint() {
                Ok(Self::U64(r?))
            } else if let Some(r) = p.try_parse_int() {
                Ok(Self::I64(r?))
            } else if let Some(r) = p.try_parse_float() {
                Ok(Self::F64(r?))
            } else if let Some(r) = p.try_parse_string() {
                Ok(Self::String(r?))
            } else if p.try_start_list().is_some() {
                let mut vec = Vec::new();
                loop {
                    if p.peek_end_list() {
                        p.end_list()?;
                        break;
                    }
                    if p.peek_eof() {
                        return Err(p.error(ParseErrorKind::ExpectedListEnd));
                    }
                    vec.push(Value::parse(p)?);
                }
                Ok(Value::List(vec))
            } else if p.try_start_map().is_some() {
                let mut map = HashMap::new();
                loop {
                    if p.peek_end_map() {
                        p.end_map()?;
                        break;
                    }
                    if p.peek_eof() {
                        return Err(p.error(ParseErrorKind::ExpectedMapEnd));
                    }
                    let path = p.parse_path()?;
                    p.map_delimiter()?;
                    let mut value = Value::parse(p)?;
                    for x in path.path.into_iter().rev() {
                        let mut map = HashMap::with_capacity(1);
                        map.insert(x, value);
                        value = Value::Map(map)
                    }
                    map.insert(path.key, value);
                }
                Ok(Value::Map(map))
            } else {
                Err(p.error(ParseErrorKind::UnknownToken))
            }
        }

        fn parse_file(p: &mut Parser) -> Result<Self, ParseError> {
            let mut map = HashMap::new();
            loop {
                if p.peek_eof() {
                    break;
                }
                let path = p.parse_path()?;
                p.map_delimiter()?;
                let mut value = Value::parse(p)?;
                for x in path.path.into_iter().rev() {
                    let mut map = HashMap::with_capacity(1);
                    map.insert(x, value);
                    value = Value::Map(map)
                }
                map.insert(path.key, value);
            }
            Ok(Value::Map(map))
        }
    }
}
