//! This crate contains a module for [Orthrus](https://crates.io/crates/orthrus) that adds support
//! for the Yaz0 compression format used in various games on the Nintendo GameCube, Wii, Wii U, and
//! Switch.
//!
//! Because the Yaz0 format is so lightweight, this crate is designed to not have any persistent
//! data. It takes in data, and will return the de/compressed data contained inside.
//!
//! # Format
//! The Yaz0 format is an extremely simple [run-length encoding](https://w.wiki/7Ecx) format. The
//! header is as follows, in big-endian format:
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | 0x0 | Magic number | u8\[4\] | Unique identifier ("Yaz0") to let us know we're reading a Yaz0-compressed file. |
//! | 0x4 | Output size  | u32     | The size of the decompressed data, needed for the output buffer. |
//! | 0x8 | Alignment    | u32     | Specifies the alignment needed for the output buffer. Non-zero starting with Wii U. |
//! | 0xC | Padding      | u8\[4\] | Alignment to a 0x10 byte boundary. Always 0. |
//!
//! The run-length decompression is as follows, ran in a loop until you write enough bytes to fill
//! the output buffer:
//!
//! * Read one byte from the input, which is 8 flag bits from high to low.
//! * For each flag bit, if it is a 1, copy one byte from the input to the output.
//! * If it is a 0, copy bytes from earlier in the output buffer:
//!     * Read two bytes from the input.
//!     * Get the first nibble (code >> 12). If it is 0, read one more byte and add 18 (0x12).
//!       Otherwise, add 2 to the nibble. Use that as the number of bytes to copy.
//!     * Add 1 to the lower nibbles (code & 0xFFF) and treat that as how far back in the buffer to
//!       read, from the current position.
//!     * **Note that the count can overlap with the destination, and needs to be copied one byte at
//!       a time.**
//!     * Copy that amount of bytes from back in the buffer to the current position.
use core::fmt::Display;
use core::str::from_utf8;
use std::io::prelude::*;
use std::path::Path;

use orthrus_core::prelude::*;

/// Unique identifier that tells us if we're reading a Yaz0-compressed file
pub const MAGIC: [u8; 4] = *b"Yaz0";

/// Loads the file at `path` and tries to decompress it as a Yaz0 file.
///
/// # Errors
/// This function will return an error if `path` does not exist, if unable to read file metadata, if
/// it does not contain a valid Yaz0 file or if there is an unexpected end-of-file.
///
/// # Panics
/// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
pub fn decompress_from_path<P: AsRef<Path> + Display>(path: P) -> Result<DataCursor> {
    log::info!("Loading Yaz0 file from {path}");
    let input = DataCursor::from_path(&path, Endian::Big)?;
    crate::decompress_from(input)
}

/// Tries to decompress a [`DataCursor`] as a Yaz0 file.
///
/// # Errors
/// This function will return an error if it does not contain valid Yaz0 data, or if there is an
/// unexpected end-of-file.
///
/// # Panics
/// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
pub fn decompress_from(mut input: DataCursor) -> Result<DataCursor> {
    log::info!("Reading Yaz0 header");
    let mut magic = [0u8; 4];
    input.read_exact(&mut magic)?;

    if magic != crate::MAGIC {
        let error = Error::InvalidMagic {
            expected: format!("{:?}", from_utf8(&crate::MAGIC)?).into(),
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
    let mut output = DataCursor::new(vec![0u8; dec_size as usize], Endian::Big);

    log::debug!("Starting Yaz0 decompression");
    //Perform the actual decompression
    crate::decompress_into(&mut input, &mut output)?;
    log::debug!("Finished Yaz0 decompression");

    //If we've gotten this far, output contains valid decompressed data
    Ok(output)
}

/// Decompresses a Yaz0 file into the output buffer.
///
/// This function makes no guarantees about the validity of the Yaz0 stream. It requires that
/// `input` is a valid Yaz0 file including the header, and that `output` is large enough to write
/// the decompressed data into.
///
/// # Errors
/// This function will return a [`DataCursorError`] if read/write fails.
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

            output.copy_range_within(output.position() - back, size)?;
        } else {
            //copy one byte
            input.copy_byte_to(output)?;
        }

        mask >>= 1;
    }
    Ok(())
}

// note to self: for compression algo, check if the min size is even possible (2), check max
// (0x111), anywhere in the 0x1000 runback and then bisect until we find the minimum (0x88, 0x44,
// 0xAA, etc)
