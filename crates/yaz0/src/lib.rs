//! This crate is a module for [Orthrus](https://crates.io/crates/orthrus) that adds support for the
//! Yaz0 compression format used for various first-party games on the Nintendo Wii, Wii U, and
//! Switch.
//!
//! # Format
//! The Yaz0 format is an extremely simple [run-length
//! encoding](https://wikipedia.org/wiki/Run-length_encoding) format. The header is as follows, in
//! big-endian format:
//!
//! | Offset | Field | Type | Length | Notes |
//! |---|---|---|---|---|
//! | 0 | Magic number | u8\[4\] | 4 | The unique identifier to let us know we're reading a Yaz0-compressed file ("Yaz0"). |
//! | 4 | Decompressed size | u32 | 4 | This is the length of the data after being decompressed, to know when all data has been written. |
//! | 8 | Alignment | u32 | 4 | This specifies the alignment needed for the output buffer. Only used starting with Wii U. |
//! | 12 | Padding | u8\[4\] | 4 | Align the compressed data to a 0x10 byte boundary. Always 0. |
//!
//! In order to decompress the data, loop through the following until you write enough bytes to
//! equal the decompressed size:
//!
//! * Read one byte, which will be treated as 8 flag bits from high to low.
//!
//! * For each flag bit, if it is a 1, copy one byte from the input.
//!
//! * If it is a 0, we need to copy bytes from earlier in the output buffer.
//!     * Read two bytes.
//!     * Get the first nibble (code >> 12). If it is 0, read one more byte, add 18 (0x12), and use
//!       that as the number of bytes to copy. Otherwise, add 2 to the nibble.
//!     * Add 1 to the lower nibbles (code & 0xFFF) and treat that as how far back in the buffer to
//!       read from.
//!     * **Note that the count can overlap with the destination, so you have to copy one byte at a
//!       time in a loop.**
//!     * For however many bytes need to be copied, copy one byte from (output - back) to output.
use std::io::prelude::*;
use std::path::Path;

use orthrus_helper::{DataCursor, Result};

/// Loads the file at `path` and tries to decompress it as a Yaz0 file.
/// 
/// # Errors
/// Returns an [IOError](orthrus_helper::Error::Io) if `path` does not exist, or read/write fails.
/// 
/// # Panics
/// Panics if the Yaz0 stream is malformed and tries to read past file bounds.
pub fn decompress_from_path<P>(path: P) -> Result<DataCursor>
where
    P: AsRef<Path>,
{
    //acquire file data, return an error if we can't
    let mut input = DataCursor::from_path(path)?;

    //read header from the buffer
    let _magic = input.read_u32_be()?; //"Yaz0"
    let dec_size = input.read_u32_be()?;
    let _alignment = input.read_u32_be()?; //0 on GC/Wii files

    //allocate decompression buffer
    let mut output = DataCursor::new(vec![0u8; dec_size as usize]);

    //perform the actual decompression
    decompress_into(&mut input, &mut output)?;

    //if we've gotten this far, buffer is the valid decompressed data
    Ok(output)
}

/// Decompresses a Yaz0 file into the output buffer.
///
/// This function makes no guarantees about the validity of the Yaz0 stream. It requires that input
/// is a valid Yaz0 file including the header, and that output is large enough to write the
/// decompressed data into.
/// 
/// # Errors
/// Returns a [`std::io::Error`] if read/write fails.
/// 
/// # Panics
/// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
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
            let code = input.read_u16_be()?;

            let back = usize::from((code & 0xFFF) + 1);
            let size = match code >> 12 {
                0 => usize::from(input.read_u8()?) + 0x12,
                n => usize::from(n) + 2,
            };

            //the ranges can overlap so we need to copy byte-by-byte
            let mut temp = [0u8; 1];
            let position = output.position();
            for n in position..position + size {
                output.set_position(n - back);
                temp[0] = output.read_u8()?;
                output.set_position(n);
                output.write_all(&temp)?;
            }
        } else {
            //copy one byte
            input.copy_byte(output)?;
        }

        mask >>= 1;
    }
    Ok(())
}

// note to self: for compression algo, check if the min size is even possible (2), check max
// (0x111), anywhere in the 0x1000 runback and then bisect until we find the minimum (0x88, 0x44,
// 0xAA, etc)
