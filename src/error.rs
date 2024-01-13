use std::{
    fmt::{self, Display},
    result, io,
};

use serde::{de, ser};

use crate::parse::ParseError;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    kind: Box<ErrorKind>,
}

impl Error {
    pub fn custom(msg: impl Display) -> Self {
        Self {
            kind: ErrorKind::Message(msg.to_string()).into(),
        }
    }

    pub fn parse(err: ParseError) -> Self {
        Self {
            kind: ErrorKind::Parse(err).into(),
        }
    }

    pub fn io(err: io::Error) -> Self {
        Self {
            kind: ErrorKind::Io(err).into()
        }
    }
}

impl From<ParseError> for Error {
    fn from(value: ParseError) -> Self {
        Self::parse(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::io(value)
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error {
            kind: ErrorKind::Message(msg.to_string()).into(),
        }
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error {
            kind: ErrorKind::Message(msg.to_string()).into(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub enum ErrorKind {
    Message(String),
    Parse(ParseError),
    Io(io::Error),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(v) => write!(f, "{v}"),
            Self::Parse(v) => write!(f, "{v}"),
            Self::Io(v) => write!(f, "{v}"),
        }
    }
}
