/// Adds support for the Resource Pack (PCK) format used by the Godot game engine.
/// 
/// This module is designed to assume little-endian files, as Godot will almost always be on a little-endian
/// platform, but it does have the capability to save a file as big-endian. If you encounter this, please let
/// me know!
/// 
/// # Format
/// The PCK format is designed to be easily parsable, and able to be embedded "inside" an executable for ease
/// of distribution. There are multiple paths to locating a PCK inside a provided file. First, it will check
/// if the file just a plain PCK by checking for the "GDPC" magic. If it doesn't find that, it will try to
/// open the file as an executable and find a section labeled "pck". If it can't find that, it will check the
/// last 4 bytes of the file. If it matches the "GDPC" magic, it will load the mini-header at the end of the
/// file to obtain the relative offset to the start of the PCK.

use orthrus_core::prelude::*;
use orthrus_windows::pe::PortableExecutable;
use snafu::prelude::*;

#[cfg(feature = "std")]
use std::path::Path;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Filesystem Error {}", source))]
    FileError { source: std::io::Error },

    /// Thrown if trying to read the file out of its current bounds.
    #[snafu(display("Reached the end of the current stream!"))]
    EndOfFile,

    /// Thrown if the header contains a magic number other than "pmf\0\n\r".
    #[snafu(display("Invalid Magic! Expected {:?}.", ResourcePack::MAGIC))]
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

impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        Error::FileError { source: error }
    }
}

pub struct ResourcePack;

impl ResourcePack {
    /// Unique identifier that tells us if we're reading a Godot PCK archive.
    pub const MAGIC: [u8; 4] = *b"GDPC";

    #[inline]
    #[cfg(feature = "std")]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, self::Error> {
        fn inner(path: &Path) -> Result<ResourcePack, self::Error> {
            let data = std::fs::read(path)?;
            ResourcePack::load(data)
        }
        inner(path.as_ref())
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self, self::Error> {
        fn inner(input: Box<[u8]>) -> Result<ResourcePack, self::Error> {
            let asdf = PortableExecutable::new(&input).unwrap();
            Ok(ResourcePack {})
        }
        inner(input.into())
    }
}
