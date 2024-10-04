use orthrus_core::prelude::*;
use snafu::prelude::*;

/// Error conditions for when working with NintendoWare files.
#[derive(Debug, Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    /// Thrown when trying to open a file or folder that doesn't exist.
    #[snafu(display("Unable to find file/folder!"))]
    NotFound,
    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    /// Thrown when unable to open a file or folder.
    #[snafu(display("No permissions to open file/folder!"))]
    PermissionDenied,
    /// Thrown if the header contains a magic number other than what's expected.
    #[snafu(display("Invalid Magic! Expected {:?}.", expected))]
    InvalidMagic { expected: [u8; 4] },
    /// Thrown when encountering unexpected values.
    #[snafu(display("Unexpected value encountered! Reason: {}", reason))]
    InvalidData { reason: &'static str },
    /// Thrown if UTF-8 validation fails when converting a string.
    #[snafu(display("Invalid UTF-8 String!"))]
    InvalidUtf8,
    /// Thrown if unable to find a specific node in the tree.
    #[snafu(display("Node not found!"))]
    NodeNotFound,
}
pub(crate) type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::UnexpectedEof => Self::EndOfFile,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            kind => {
                panic!("Unexpected std::io::error: {kind}! Something has gone horribly wrong")
            }
        }
    }
}

impl From<DataError> for Error {
    #[inline]
    fn from(error: DataError) -> Self {
        match error {
            DataError::EndOfFile => Self::EndOfFile,
            _ => panic!("Unexpected data::error! Something has gone horribly wrong"),
        }
    }
}
