use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("time::IndeterminateOffset error: {0}")]
    TimeInvalidOffset(#[from] time::error::IndeterminateOffset),

    #[error("time::Format error: {0}")]
    TimeInvalidFormat(#[from] time::error::Format),

    #[error("time::ComponentRange error: {0}")]
    TimeInvalidRange(#[from] time::error::ComponentRange),

    #[error("log::SetLoggerError error: {0}")]
    LogInvalidLogger(#[from] log::SetLoggerError),

    #[error("Invalid magic number: expected {expected}, got {got}")]
    InvalidMagic { expected: String, got: String },

    #[error("Unknown version: expected {expected}, got {got}")]
    UnknownVersion { expected: String, got: String },
}

pub type Result<T> = std::result::Result<T, Error>;
