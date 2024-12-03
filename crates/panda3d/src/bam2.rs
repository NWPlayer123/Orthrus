#[cfg(feature = "std")]
use std::path::Path;

use snafu::prelude::*;

use orthrus_core::prelude::*;

use crate::common::Version;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Filesystem Error {}", source))]
    ReadFile { source: std::io::Error },

    /// Thrown if trying to read the file out of its current bounds.
    #[snafu(display("Reached the end of the current stream!"))]
    EndOfFile,

    /// Thrown if the header contains a magic number other than "pmf\0\n\r".
    #[snafu(display("Invalid Magic! Expected {:?}.", BinaryAsset::MAGIC))]
    InvalidMagic,
}

impl From<DataError> for Error {
    #[inline]
    fn from(error: DataError) -> Self {
        match error {
            DataError::EndOfFile => Self::EndOfFile,
            _ => todo!(),
        }
    }
}

#[allow(dead_code)]
pub struct BinaryAsset {
    data: DataCursor,
}

impl BinaryAsset {
    /// Latest revision of the BAM format. For more info, see [here](self#revisions).
    pub const CURRENT_VERSION: Version = Version { major: 6, minor: 45 };
    /// Unique identifier that tells us if we're reading a Panda3D Binary Object.
    pub const MAGIC: [u8; 6] = *b"pbj\0\n\r";
    /// Earliest supported revision of the BAM format. For more info, see [here](self#revisions).
    pub const MINIMUM_VERSION: Version = Version { major: 6, minor: 14 };
    
    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, self::Error> {
        fn inner(path: &Path) -> Result<BinaryAsset, self::Error> {
            let data = std::fs::read(path).context(ReadFileSnafu)?;
            BinaryAsset::load(data)
        }
        inner(path.as_ref())
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self, self::Error> {
        fn inner(input: Box<[u8]>) -> Result<BinaryAsset, self::Error> {
            let mut data = DataCursor::new(input, Endian::Little);
            data.read_u32()?;
            Err(Error::EndOfFile)
        }
        inner(input.into())
    }
}
