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

//This is needed for macros
#![allow(unused_assignments)]
use core::fmt::Display;
use std::path::Path;

use orthrus_core::prelude::*;
use snafu::prelude::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to find file/folder!"))]
    NotFound,
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    #[snafu(display("Invalid Size Encountered!"))]
    InvalidSize,
    #[snafu(display("File too large to fit into u32::MAX!"))]
    FileTooBig,
    #[snafu(display("Invalid Magic! Expected {expected:?}"))]
    InvalidMagic { expected: [u8; 4] },
}
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Error::NotFound,
            std::io::ErrorKind::UnexpectedEof => Error::EndOfFile,
            _ => panic!("Unexpected std::io::error! Something has gone horribly wrong"),
        }
    }
}

impl From<data::Error> for Error {
    fn from(error: data::Error) -> Self {
        match error {
            data::Error::EndOfFile => Error::EndOfFile,
            data::Error::InvalidSize => Error::InvalidSize,
        }
    }
}

macro_rules! read_u8 {
    ($data:expr, $pos:expr) => {{
        let value = $data[$pos];
        $pos += 1;
        value
    }};
}

macro_rules! read_u16 {
    ($data:expr, $pos:expr) => {{
        let value = u16::from_be_bytes([$data[$pos], $data[$pos + 1]]);
        $pos += 2;
        value
    }};
}

macro_rules! read_u32 {
    ($data:expr, $pos:expr) => {{
        let value = u32::from_be_bytes([
            $data[$pos],
            $data[$pos + 1],
            $data[$pos + 2],
            $data[$pos + 3],
        ]);
        $pos += 4;
        value
    }};
}

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
#[cfg(feature = "std")]
pub fn decompress_from_path<P: AsRef<Path> + Display>(path: P) -> Result<DataCursor> {
    log::info!("Loading Yaz0 file from {path}");
    let input = std::fs::read(path)?;
    self::decompress_from(&input)
}

/// Tries to decompress a [`DataCursor`] as a Yaz0 file.
///
/// # Errors
/// This function will return an error if it does not contain valid Yaz0 data, or if there is an
/// unexpected end-of-file.
///
/// # Panics
/// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
pub fn decompress_from(data: &[u8]) -> Result<DataCursor> {
    log::info!("Reading Yaz0 header");
    let mut pos: usize = 0;

    let magic = &data[pos..pos + 4];
    pos += 4;

    if magic != self::MAGIC {
        let error = Error::InvalidMagic {
            expected: self::MAGIC,
        };
        log::error!("{error}");
        return Err(error);
    }

    let dec_size = read_u32!(data, pos);
    let alignment = read_u32!(data, pos); //0 on GC/Wii files
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
    self::decompress_into(data, &mut output)?;
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
fn decompress_into(input: &[u8], output: &mut DataCursor) -> Result<()> {
    let mut mask: u8 = 0;
    let mut flags: u8 = 0;

    let mut inputpos: usize = 0x10;
    output.set_position(0);

    while output.position() < output.len() {
        //out of flag bits for RLE, load in a new byte
        if mask == 0 {
            mask = 1 << 7;
            flags = read_u8!(input, inputpos);
        }

        if (flags & mask) == 0 {
            //do RLE copy
            let code = read_u16!(input, inputpos);

            let back = usize::from((code & 0xFFF) + 1);
            let size = match code >> 12 {
                0 => usize::from(read_u8!(input, inputpos)) + 0x12,
                n => usize::from(n) + 2,
            };

            output.copy_within(output.position() - back, size)?;
        } else {
            //copy one byte
            output.write_u8(read_u8!(input, inputpos))?;
        }

        mask >>= 1;
    }
    Ok(())
}

pub enum CompressionAlgo {
    MarioKartWii, //eggCompress
}

#[cfg(feature = "std")]
pub fn compress_from_path<P: AsRef<Path> + Display>(
    path: P,
    algo: CompressionAlgo,
    align: u32,
) -> Result<DataCursor> {
    log::info!("Loading binary file from {path}");
    let input = std::fs::read(path)?;
    self::compress_from(&input, algo, align)
}

/// This function compresses the input data using a given compression algorithm.
///
/// # Warnings
/// Alignment should be zero on GameCube and Wii platforms, and non-zero on Wii U and Switch
/// platforms. This function does not try to discern what platform the file is being compressed for,
/// or if the alignment is valid.
pub fn compress_from(input: &[u8], algo: CompressionAlgo, align: u32) -> Result<DataCursor> {
    ensure!(input.len() <= u32::MAX as usize, FileTooBigSnafu);

    //Assume 0x10 header, every byte is a copy, and include flag bytes (rounded up)
    let worst_possible_size: usize = 0x10 + input.len() + input.len().div_ceil(8);
    let mut output = DataCursor::new(vec![0u8; worst_possible_size], Endian::Big);

    //Don't try to write the header in the hot path
    output.write_u32(u32::from_be_bytes(*b"Yaz0"))?; // "Yaz0" magic
    output.write_u32(input.len() as u32)?; // Output size for decompression algo
    output.write_u32(align)?; // Alignment, platform-specific
    output.set_position(0x10);

    let output_size = match algo {
        CompressionAlgo::MarioKartWii => compress_into_mkw(input, &mut output),
    };

    Ok(output.shrink_to(output_size as usize)?)
}

#[inline(never)]
fn compress_into_mkw(input: &[u8], output: &mut [u8]) -> usize {
    log::info!("Working on MKW compression!");
    let mut input_pos: usize;
    let mut output_pos: usize;
    let mut flag_byte_pos: usize;
    let mut flag_byte_shift: u8;

    input_pos = 0;
    output_pos = 0x11;
    flag_byte_pos = 0x10;
    flag_byte_shift = 0x80;

    while input_pos < input.len() {
        let mut first_match_offset: usize;
        let mut first_match_len: usize;
        (first_match_offset, first_match_len) = find_match(input, input_pos);
        log::info!("First Match Offset: {}, First Match Length: {}", first_match_offset, first_match_len);
        if first_match_len > 2 {
            let second_match_offset: usize;
            let second_match_len: usize;
            (second_match_offset, second_match_len) = find_match(input, input_pos + 1);
            log::info!("Second Match Offset: {}, Second Match Length: {}", second_match_offset, second_match_len);
            if first_match_len + 1 < second_match_len {
                //TODO: merge this and the outer else?
                output[flag_byte_pos] |= flag_byte_shift;
                flag_byte_shift >>= 1;
                output[output_pos] = input[input_pos];
                input_pos += 1;
                output_pos += 1;
                if flag_byte_shift == 0 {
                    flag_byte_shift = 0x80;
                    flag_byte_pos = output_pos;
                    output[output_pos] = 0;
                    output_pos += 1;
                }
                first_match_len = second_match_len;
                first_match_offset = second_match_offset;
            }
            first_match_offset = input_pos - first_match_offset - 1;
            if first_match_len < 18 {
                first_match_offset |= (first_match_len - 2) << 12;
                log::info!("Writing two byte RLE! {}, {}", first_match_offset, output_pos);
                output[output_pos] = (first_match_offset >> 8) as u8;
                output[output_pos + 1] = (first_match_offset) as u8;
                output_pos += 2;
            } else {
                log::info!("Writing three byte RLE! {}, {}, {}", first_match_offset, first_match_len, output_pos);
                output[output_pos] = (first_match_offset >> 8) as u8;
                output[output_pos + 1] = (first_match_offset) as u8;
                output[output_pos + 2] = (first_match_len - 18) as u8;
                output_pos += 3;
            }
            input_pos += first_match_len;
        } else {
            output[flag_byte_pos] |= flag_byte_shift;
            output[output_pos] = input[input_pos];
            input_pos += 1;
            output_pos += 1;
        }

        flag_byte_shift >>= 1;
        if flag_byte_shift == 0 {
            flag_byte_shift = 0x80;
            flag_byte_pos = output_pos;
            output[output_pos] = 0;
            output_pos += 1;
        }
    }

    output_pos
}

#[inline(never)]
fn find_match(input: &[u8], input_pos: usize) -> (usize, usize) {
    log::info!("Trying to find a match!");
    let mut window: usize = if input_pos > 4096 {
        input_pos - 4096
    } else {
        0
    };
    let mut window_size = 3;
    let max_match_size = if (input.len() - input_pos) <= 273 {
        input.len() - input_pos
    } else {
        273
    };
    if max_match_size < 3 {
        return (0, 0);
    }

    let mut window_offset: usize = 0;
    let mut found_match_offset: usize = 0; //potentially uninitialized in C++ version

    while window < input_pos && {
        window_offset = search_window(
            &input[input_pos..input_pos + window_size],
            &input[window..input_pos + window_size],
        );
        window_offset < input_pos - window
    } {
        while window_size < max_match_size {
            if input[window + window_offset + window_size] != input[input_pos + window_size] {
                break;
            }
            window_size += 1;
        }
        if window_size == max_match_size {
            return (window + window_offset, max_match_size);
        }
        found_match_offset = window + window_offset;
        window_size += 1;
        window += window_offset + 1;
    }

    (
        found_match_offset,
        if window_size > 3 { window_size - 1 } else { 0 },
    )
}

#[inline(never)]
fn search_window(needle: &[u8], haystack: &[u8]) -> usize {
    /*log::info!(
        "needleSize: {}, haystackSize: {}",
        needle.len(),
        haystack.len()
    );*/
    let mut it_haystack: usize;
    let mut it_needle: usize;

    if needle.len() > haystack.len() {
        return haystack.len();
    }
    let skip_table = compute_skip_table(needle);

    it_haystack = needle.len() - 1;
    'outer: loop {
        while needle[needle.len() - 1] != haystack[it_haystack] {
            it_haystack += skip_table[haystack[it_haystack] as usize];
        }
        it_haystack -= 1;
        it_needle = needle.len() - 2;

        let remaining_bytes: usize = it_needle;
        for _ in 0..=remaining_bytes {
            if haystack[it_haystack] != needle[it_needle] {
                let mut skip: usize = skip_table[haystack[it_haystack] as usize];
                if needle.len() - it_needle > skip {
                    skip = needle.len() - it_needle;
                }
                it_haystack += skip;
                continue 'outer;
            }
            it_haystack -= 1;
            it_needle -= 1;
        }
        //log::info!("Returning {}", it_haystack + 1);
        return it_haystack + 1;
    }
}

#[inline(never)]
fn compute_skip_table(needle: &[u8]) -> [usize; 256] {
    let mut table = [needle.len(); 256];
    for i in 0..needle.len() {
        table[needle[i] as usize] = needle.len() - i - 1;
    }
    //log::info!("{table:?}");
    table
}

/*
#[inline(never)]
fn compress_into_mkw(input: &[u8], output: &mut [u8]) -> usize {
    let mut input_pos: usize;
    let mut output_pos: usize;
    let mut flag_byte_pos: usize;
    let mut flag_byte_shift: u8;

    input_pos = 0;
    output_pos = 0x11;
    flag_byte_pos = 0x10;
    flag_byte_shift = 0x80;

    while input_pos < input.len() {
        let mut first_match_offset: usize;
        let mut first_match_len: usize;
        (first_match_offset, first_match_len) = find_match(input, input_pos);
        if first_match_len > 2 {
            let second_match_offset: usize;
            let second_match_len: usize;
            (second_match_offset, second_match_len) = find_match(input, input_pos + 1);
            if first_match_len + 1 < second_match_len {
                //TODO: merge this and the outer else?
                output[flag_byte_pos] |= flag_byte_shift;
                flag_byte_shift >>= 1;
                output[output_pos] = input[input_pos];
                input_pos += 1;
                output_pos += 1;
                if flag_byte_shift == 0 {
                    flag_byte_shift = 0x80;
                    flag_byte_pos = output_pos;
                    output[output_pos] = 0;
                    output_pos += 1;
                }
                first_match_len = second_match_len;
                first_match_offset = second_match_offset;
            }
            first_match_offset = input_pos - first_match_offset - 1;
            if first_match_offset < 18 {
                first_match_offset |= (first_match_len - 2) << 12;
                output[output_pos] = (first_match_offset >> 8) as u8;
                output[output_pos + 1] = (first_match_offset) as u8;
                output_pos += 2;
            } else {
                output[output_pos] = (first_match_offset >> 8) as u8;
                output[output_pos + 1] = (first_match_offset) as u8;
                output[output_pos + 2] = (first_match_len - 18) as u8;
                output_pos += 3;
            }
            input_pos += first_match_len;
        } else {
            output[flag_byte_pos] |= flag_byte_shift;
            output[output_pos] = input[input_pos];
            input_pos += 1;
            output_pos += 1;
        }

        flag_byte_shift >>= 1;
        if flag_byte_shift == 0 {
            flag_byte_shift = 0x80;
            flag_byte_pos = output_pos;
            output[output_pos] = 0;
            output_pos += 1;
        }
    }

    output_pos
}

#[inline(never)]
fn find_match(input: &[u8], input_pos: usize) -> (usize, usize) {
    let mut window: usize = if input_pos > 4096 {
        input_pos - 4096
    } else {
        0
    };
    let mut window_size = 3;
    let max_match_size = if (input.len() - input_pos) <= 273 {
        input.len() - input_pos
    } else {
        273
    };
    if max_match_size < 3 {
        return (0, 0);
    }

    let mut window_offset: usize = 0;
    let mut found_match_offset: usize = 0; //potentially uninitialized in C++ version

    while window < input_pos {
        window_offset = search_window(
            &input[input_pos..input_pos + window_size],
            &input[window..input_pos + window_size],
        );
        if window_offset < input_pos - window {
            while window_size < max_match_size {
                if input[window + window_offset + window_size] != input[input_pos + window_size] {
                    break;
                }
                window_size += 1;
            }
            if window_size == max_match_size {
                return (window + window_offset, max_match_size);
            }
            found_match_offset = window + window_offset;
            window_size += 1;
            window += window_offset + 1;
        }
    }

    (
        found_match_offset,
        if window_size > 3 { window_size - 1 } else { 0 },
    )
}

#[inline(never)]
fn search_window(needle: &[u8], haystack: &[u8]) -> usize {
    let mut it_haystack: usize;
    let mut it_needle: usize;

    if needle.len() > haystack.len() {
        return haystack.len();
    }
    let skip_table = compute_skip_table(needle);

    it_haystack = needle.len() - 1;
    loop {
        loop {
            if needle[needle.len() - 1] == haystack[it_haystack] {
                break;
            }
            it_haystack += skip_table[haystack[it_haystack] as usize];
        }
        it_haystack -= 1;
        it_needle = needle.len() - 2;

        let remaining_bytes: usize = it_needle;
        for _ in 0..=remaining_bytes {
            if haystack[it_haystack] != needle[it_needle] {
                let mut skip: usize = skip_table[haystack[it_haystack] as usize];
                if needle.len() - it_needle > skip {
                    skip = needle.len() - it_needle;
                }
                it_haystack += skip;
                continue;
            }
            it_haystack -= 1;
            it_needle -= 1;
        }
        return it_haystack + 1;
    }
}

#[inline(never)]
fn compute_skip_table(needle: &[u8]) -> [usize; 256] {
    let mut table = [needle.len(); 256];
    for i in 0..needle.len() {
        table[needle[i] as usize] = needle.len() - i - 1;
    }
    table
}
*/
