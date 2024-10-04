#[cfg(feature = "std")]
use std::path::Path;

use snafu::prelude::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not read file {path}"))]
    ReadFile { source: std::io::Error, path: String },
}

#[expect(dead_code)]
pub struct BinaryAsset {
    data: Vec<u8>,
}

impl BinaryAsset {
    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, self::Error> {
        fn inner(path: &Path) -> Result<BinaryAsset, self::Error> {
            Ok(BinaryAsset {
                data: std::fs::read(path).context(ReadFileSnafu { path: path.to_string_lossy() })?,
            })
        }
        inner(path.as_ref())
    }
}
