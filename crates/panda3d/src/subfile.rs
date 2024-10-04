//! Adds support for the Subfile format used inside of Multifiles.
//!
//! This module is mainly internal right now, as Subfiles are heavily tied to their associated
//! [`Multifile`](crate::multifile::Multifile) for the moment.
//!
//! # Format
//! Refer to the [Multifile format](crate::multifile#format) for more details.

#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

use bitflags::bitflags;
use orthrus_core::prelude::*;

use crate::common::Version;
use crate::multifile::Result;
#[cfg(not(feature = "std"))]
use crate::no_std::*;

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub(crate) struct Flags: u16 {
        const Deleted = 1 << 0;
        const IndexInvalid = 1 << 1;
        const DataInvalid = 1 << 2;
        const Compressed = 1 << 3;
        const Encrypted = 1 << 4;
        const Signature = 1 << 5;
        const Text = 1 << 6;
    }
}

/// Utility struct for handling Subfile data, for use with
/// [`Multifile`](crate::multifile::Multifile) archives. Currently only for internal use.
///
/// For more details on the Multifile format, see the [module documentation](self#format).
#[derive(Default, Debug)]
#[allow(dead_code)]
pub struct Subfile {
    pub(crate) offset: u32,
    pub(crate) length: u32,
    pub(crate) flags: Flags,
    pub(crate) timestamp: u32,
    pub(crate) filename: String,
}

impl Subfile {
    /// Parses the header of a given [`Subfile`] and returns a new instance with its data.
    ///
    /// # Warnings
    /// This function assumes that `input` is positioned at the start of a valid [`Subfile`] header.
    /// It will happily try to parse whatever is given to it, which can lead to the filename going
    /// out of bounds. Be careful!
    ///
    /// # Errors
    /// Returns [`EndOfFile`] if it tries to read out of bounds.
    #[allow(dead_code)]
    pub(crate) fn load<T: ReadExt>(input: &mut T, version: Version) -> Result<Self> {
        let offset = input.read_u32()?;
        let data_length = input.read_u32()?;
        let flags = Flags::from_bits_truncate(input.read_u16()?);

        let length = match flags.intersects(Flags::Compressed | Flags::Encrypted) {
            true => input.read_u32()?,
            false => data_length,
        };

        let timestamp = match version.minor >= 1 {
            true => input.read_u32()?,
            false => 0,
        };

        let name_length = input.read_u16()?;
        let mut filename = String::with_capacity(name_length.into());
        for c in &*input.read_slice(name_length.into())? {
            filename.push((255 - *c).into());
        }

        Ok(Self { offset, length, flags, timestamp, filename })
    }

    /// Writes the [`Subfile`] data to disk, using the data from the associated [`Multifile`].
    ///
    /// # Errors
    /// Returns an error if unable to create the necessary directories, or unable to create a file
    /// to write to. See [`create_dir_all`](std::fs::create_dir_all) and [`write`](std::fs::write).
    #[cfg(feature = "std")]
    #[inline]
    pub(crate) fn write_file<P: AsRef<Path>>(&mut self, data: &[u8], output: P) -> Result<()> {
        let mut path = PathBuf::from(output.as_ref());
        path.push(&self.filename);

        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        std::fs::write(path, data)?;
        Ok(())
    }
}
