//! This crate contains a module for [Orthrus](https://crates.io/crates/orthrus) that adds support
//! for the Yaz0 compression format used for various games on the Nintendo Wii, Wii U, and Switch.
//!
//! # Format
//! The Yaz0 format is an extremely simple [run-length encoding](https://w.wiki/7Ecx) format. The
//! header is as follows, in big-endian format:
//!
//! | Offset | Field | Type | Notes |
//! |---|---|---|---|---|
//! | 0x0 | Magic number | u8\[4\] | The unique identifier to let us know we're reading a Yaz0-compressed file ("Yaz0"). |
//! | 0x4 | Decompressed size | u32 | This is the size needed for the output buffer. |
//! | 0x8 | Alignment | u32 | This specifies the alignment needed for the output buffer. Only used starting with Wii U. |
//! | 0xC | Padding | u8\[4\] | Align the compressed data to a 0x10 byte boundary. Always 0. |
//!
//! In order to decompress the data, loop through the following until you write enough bytes to
//! equal the decompressed size:
//!
//! * Read one byte from the input, which is 8 flag bits from high to low.
//!
//! * For each flag bit, if it is a 1, copy one byte from the input  .
//!
//! * If it is a 0, copy bytes from earlier in the output buffer.
//!     * Read two bytes from the input.
//!     * Get the first nibble (code >> 12). If it is 0, read one more byte and add 18 (0x12).
//!       Otherwise, add 2 to the nibble. Use that as the number of bytes to copy.
//!     * Add 1 to the lower nibbles (code & 0xFFF) and treat that as how far back in the buffer to
//!       read from.
//!     * **Note that the count can overlap with the destination, and needs to be copied one byte at
//!       a time.**
//!     * Copy that amount of bytes from back in the buffer to the current location.
use core::str::from_utf8;
use std::io::prelude::*;
use std::path::Path;

use orthrus_core::prelude::*;

#[derive(Default)]
pub struct Yaz0 {
    pub data: DataCursor,
}

impl Yaz0 {
    const MAGIC: [u8; 4] = *b"Yaz0";

    /// Loads the file at `path` and tries to decompress it as a Yaz0 file.
    ///
    /// # Errors
    /// This function will return an error if `path` does not exist, if it lacks permission to read
    /// the `metadata` of `path`, if unable to convert the filesize to usize, or if it reaches an
    /// unexpected end-of-file.
    ///
    /// # Panics
    /// Panics if the Yaz0 stream is malformed and tries to read past file bounds.
    pub fn from_path<P: AsRef<Path> + std::fmt::Display>(path: P) -> Result<Self> {
        log::info!("Loading Yaz0 file from {path}");
        let mut input = DataCursor::from_path(&path, Endian::Big)?;

        let mut magic = [0u8; 4];
        input.read_exact(&mut magic)?;

        if magic != Self::MAGIC {
            let error = crate::Error::InvalidMagic {
                expected: format!("{:?}", from_utf8(&Self::MAGIC)?).into(),
            };
            log::error!("{}", error);
            return Err(error);
        }

        let dec_size = input.read_u32()?;
        let alignment = input.read_u32()?; //0 on GC/Wii files
        log::info!(
            "Output Size: {dec_size:#X}{}",
            if alignment == 0 {
                String::new()
            } else {
                format!(" | Alignment: {alignment:#X}")
            }
        );

        //Allocate decompression buffer
        let mut output = Self {
            data: DataCursor::new(vec![0u8; dec_size as usize], Endian::Big),
        };

        log::debug!("Starting decompression for {path}");
        //Perform the actual decompression
        Self::decompress_into(&mut input, &mut output.data)?;
        log::debug!("Finished decompression for {path}");

        //If we've gotten this far, buffer is the valid decompressed data
        Ok(output)
    }

    /// Decompresses a Yaz0 file into the output buffer.
    ///
    /// This function makes no guarantees about the validity of the Yaz0 stream. It requires that
    /// input is a valid Yaz0 file including the header, and that output is large enough to write
    /// the decompressed data into.
    ///
    /// # Errors
    /// Returns a [`std::io::Error`] if read/write fails.
    ///
    /// # Panics
    /// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
    #[inline(never)]
    fn decompress_into(input: &mut DataCursor, output: &mut DataCursor) -> Result<()> {
        let mut mask: u8 = 0;
        let mut flags: u8 = 0;

        input.set_position(0x10);
        output.set_position(0);

        while output.position() < output.len() {
            //out of flag bits for RLE, load in a new byte
            if mask == 0 {
                mask = 1 << 7;
                flags = input.read_u8()?;
            }

            if (flags & mask) == 0 {
                //do RLE copy
                let code = input.read_u16()?;

                let back = usize::from((code & 0xFFF) + 1);
                let size = match code >> 12 {
                    0 => usize::from(input.read_u8()?) + 0x12,
                    n => usize::from(n) + 2,
                };

                //the ranges can overlap so we need to copy byte-by-byte
                let position = output.position();
                output.copy_range_within(position - back, size)?;
            } else {
                //copy one byte
                input.copy_byte_to(output)?;
            }

            mask >>= 1;
        }
        Ok(())
    }
}

// note to self: for compression algo, check if the min size is even possible (2), check max
// (0x111), anywhere in the 0x1000 runback and then bisect until we find the minimum (0x88, 0x44,
// 0xAA, etc)
