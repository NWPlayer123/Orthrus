//! Adds support for the LZ11 compression format used in first party Nintendo GBA, NDS, and Wii games.
//!
//! # Format
//! The LZ11 format is part of the Lempel-Ziv family of algorithms, which use a sliding window to copy
//! repetitive data from previously decompressed output. It has a header followed by compressed data.
//!
//! ## Header
//! The header is as follows:
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | 0x0 | Magic byte | u8 | Unique identifier (0x11) for LZ11 format. |
//! | 0x1 | Output size | u24 | The size of the decompressed output. |
//!
//! # Encoding Format
//! The encoding uses flag bytes to determine what follows:
//! * If the flag bit is 0, the next byte is copied directly.
//! * If the flag bit is 1, the next 2-4 bytes determines the lookback parameters, based on the top nibble:
//!   - If x > 1: xA BC <-------- copy x+0x1 bytes from position - (ABC + 1)
//!   - if x = 0: 0a bA BC <----- copy ab+0x11 bytes from position - (ABC + 1)
//!   - If x = 1: 1a bc dA BC <-- copy abcd+0x111 bytes from position - (ABC + 1)
//!
//! # Usage
//! This module offers the following functionality:
//! ## Decompression
//! * [`decompress_from_path`](LZ11::decompress_from_path): Provide a path, get decompressed data back
//! * [`decompress_from`](LZ11::decompress_from): Provide the input data, get decompressed data back
//! * [`decompress`](LZ11::decompress): Provide the input data and output buffer, run the decompression
//!   algorithm
//! ## Compression
//! * [`compress_from_path`](LZ11::compress_from_path): Provide a path, get compressed data back
//! * [`compress_from`](LZ11::compress_from): Provide the input data, get compressed data back
//! * [`compress`](LZ11::compress): Provide the input data and output buffer, run the compression algorithm
//! ## Utilities
//! * [`read_header`](LZ11::read_header): Returns the header information for a given LZ11 file
//! * [`worst_possible_size`](LZ11::worst_possible_size): Calculates the worst possible compression size

#[cfg(feature = "std")] use std::path::Path;

use orthrus_core::prelude::*;
use snafu::prelude::*;

#[cfg(not(feature = "std"))] use crate::no_std::*;

/// Error conditions for when reading/writing LZ11 files
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    /// Thrown if an error occurs when trying to read or write files.
    #[snafu(transparent)]
    FileError { source: std::io::Error },

    /// Thrown if an error occurs when trying to read or write data.
    #[snafu(transparent)]
    DataError { source: DataError },

    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Reached the end of the current stream!"))]
    EndOfFile,

    /// Thrown if the header contains an unexpected magic number.
    #[snafu(display("Unexpected Magic! Expected {expected:?}."))]
    InvalidMagic { expected: &'static [u8] },
}

pub struct LZ11;

impl LZ11 {
    pub const MAGIC: &'static [u8] = b"\x11";

    /// Calculates the filesize of the largest possible file that can be created for a given length.
    ///
    /// This consists of the 4 byte metadata, the length of the input file, and all flag bytes, rounded up.
    #[inline]
    pub const fn worst_possible_size(length: usize) -> usize {
        4 + length + (length + 7) / 8
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn decompress_from_path<P: AsRef<Path>>(path: P) -> Result<Box<[u8]>, self::Error> {
        let input = std::fs::read(path)?;
        Self::decompress_from(&input)
    }

    #[inline]
    pub fn decompress_from(input: &[u8]) -> Result<Box<[u8]>, self::Error> {
        ensure!(input.len() >= 4, EndOfFileSnafu);

        let mut data = DataCursor::new(input, Endian::Little);

        ensure!(data.read_exact::<1>()? == Self::MAGIC, InvalidMagicSnafu { expected: Self::MAGIC });

        let mut output = vec![0u8; data.read_u24()? as usize].into_boxed_slice();

        Self::decompress(input, &mut output)?;

        Ok(output)
    }

    #[inline]
    pub fn decompress(input: &[u8], output: &mut [u8]) -> Result<(), self::Error> {
        let mut input = DataCursorRef::new(input, Endian::Big);
        let mut output = DataCursorMut::new(output, Endian::Big);
        input.set_position(4)?;
        let mut mask = 0u8;
        let mut flags = 0u8;

        while output.position()? < output.len()? {
            // Check if we need a new flag byte
            if mask == 0 {
                flags = input.read_u8()?;
                mask = 1 << 7;
            }

            // Check what kind of copying we're doing
            if (flags & mask) == 0 {
                // Copy one byte from the input stream
                output.write_u8(input.read_u8()?)?;
            } else {
                // RLE copy from previously in the buffer
                let initial = input.read_u16()? as usize;
                let (distance, length) = match initial >> 12 {
                    0 => {
                        let input = (initial & 0xFFF) << 8 | input.read_u8()? as usize;
                        ((input & 0xFFF) + 1, (input >> 12) + 0x11)
                    }
                    1 => {
                        let input = (initial & 0xFFF) << 16 | input.read_u16()? as usize;
                        ((input & 0xFFF) + 1, (input >> 12) + 0x111)
                    }
                    n => ((initial & 0xFFF) + 1, n + 1),
                };

                // Ensure the sliding window is valid.
                let current_pos = output.position()? as usize;
                ensure!(distance <= current_pos, EndOfFileSnafu);

                let start = current_pos - distance;
                let end = start + length;
                output.copy_within(start..end, current_pos)?;
                output.set_position((current_pos + length) as u64)?;
            }

            mask >>= 1;
        }

        Ok(())
    }
}
