#[cfg(feature = "std")]
use std::path::Path;

use snafu::prelude::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Filesystem Error {}", source))]
    ReadFile { source: std::io::Error },
}

pub struct BinaryAsset {}

impl BinaryAsset {
    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, self::Error> {
        fn inner(path: &Path) -> Result<BinaryAsset, self::Error> {
            let data = std::fs::read(path).context(ReadFileSnafu)?;
            Ok(BinaryAsset::load(data)?)
        }
        inner(path.as_ref())
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self, self::Error> {
        fn inner(input: Box<[u8]>) -> Result<BinaryAsset, self::Error> {
            Ok(BinaryAsset {})
        }
        inner(input.into())
    }
}
