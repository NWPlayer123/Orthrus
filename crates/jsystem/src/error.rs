use orthrus_core::data::DataError;
use snafu::prelude::*;

/// Error conditions when working with Resource Archives.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "std")]
    #[snafu(display("Filesystem Error {source}"))]
    FileError { source: std::io::Error },

    /// Thrown if trying to read the file out of its current bounds.
    #[snafu(display("Reached the end of the current stream!"))]
    EndOfFile,

    /// Thrown if the header contains a magic number other than "RARC".
    #[snafu(display("Invalid Magic! Expected {expected:?}."))]
    InvalidMagic { expected: [u8; 4] },

    /// Thrown when encountering unexpected values.
    #[snafu(display(
        "Unexpected value encountered at position {:#X}! Reason: {}",
        position,
        reason
    ))]
    InvalidData { position: u64, reason: &'static str },
}

impl From<DataError> for Error {
    #[inline]
    fn from(error: DataError) -> Self {
        match error {
            #[cfg(feature = "std")]
            DataError::Io { source } => Self::FileError { source },
            DataError::EndOfFile => Self::EndOfFile,
            _ => todo!(),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        Error::FileError { source: error }
    }
}
