
/*
use compact_str::CompactString;
use thiserror::Error;
use x509_parser::nom;

use crate::data::DataCursorError;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    IntError(#[from] core::num::TryFromIntError),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    TimeError(#[from] Box<time::error::Error>),

    #[error(transparent)]
    LogInvalidLogger(#[from] log::SetLoggerError),

    #[error(transparent)]
    X509Error(x509_parser::error::X509Error),

    #[error(transparent)]
    DataCursorError(#[from] DataCursorError),

    #[error("Invalid magic number: expected {expected}")]
    InvalidMagic { expected: CompactString },

    #[error("Unknown version: expected {expected}")]
    UnknownVersion { expected: CompactString },

    #[error("Unexpected end-of-file encountered")]
    EndOfFile,
}

impl From<nom::Err<x509_parser::error::X509Error>> for Error {
    fn from(value: nom::Err<x509_parser::error::X509Error>) -> Self {
        Self::X509Error(value.into())
    }
}

impl From<x509_parser::error::X509Error> for Error {
    fn from(value: x509_parser::error::X509Error) -> Self {
        Self::X509Error(value)
    }
}

impl From<time::error::Error> for Error {
    fn from(err: time::error::Error) -> Self {
        Self::TimeError(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
*/
