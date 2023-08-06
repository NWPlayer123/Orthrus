use thiserror::Error;
use x509_parser::nom;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    IntError(#[from] core::num::TryFromIntError),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    TimeInvalidOffset(#[from] time::error::IndeterminateOffset),

    #[error(transparent)]
    TimeInvalidFormat(#[from] time::error::Format),

    #[error(transparent)]
    TimeInvalidRange(#[from] time::error::ComponentRange),

    #[error(transparent)]
    LogInvalidLogger(#[from] log::SetLoggerError),

    #[error(transparent)]
    X509Error(#[from] x509_parser::error::X509Error),

    #[error(transparent)]
    X509NomError(x509_parser::error::X509Error),

    #[error("Invalid magic number: expected {expected}, got {got}")]
    InvalidMagic { expected: String, got: String },

    #[error("Unknown version: expected {expected}, got {got}")]
    UnknownVersion { expected: String, got: String },

    #[error("Unexpected end-of-file encountered")]
    EndOfFile,
}

impl From<nom::Err<x509_parser::error::X509Error>> for Error {
    fn from(err: nom::Err<x509_parser::error::X509Error>) -> Self {
        Self::X509NomError(err.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
