//! Adds support for the Yaz0 compression format, used on GameCube, Wii, Wii U, and Switch.
//!
//! Because the Yaz0 format is so lightweight, this module is designed to not have any persistence.
//! It takes in data, and will return the de/compressed data contained inside.
//!
//! # Format
//! The Yaz0 format is part of the [Lempel-Ziv family of algorithms](https://w.wiki/F6n), which use
//! a "sliding window" to allow for copying repetitive data from previously in the output buffer.
//! The input stream consists of lookback+length pairs, unique bytes to copy, and "flag bytes" which
//! determine which of the two operations to do.
//!
//! # Header
//! The header is as follows, in big-endian format:
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | 0x0 | Magic number | u8\[4\] | Unique identifier ("Yaz0") to let us know we're reading a Yaz0-compressed file. |
//! | 0x4 | Output size  | u32     | The size of the decompressed data, needed for the output buffer. |
//! | 0x8 | Alignment    | u32     | Specifies the alignment needed for the output buffer. Non-zero starting with Wii U. |
//! | 0xC | Padding      | u8\[4\] | Alignment to a 0x10 byte boundary. Always 0. |
//!
//! # Decompression
//! The decompression algorithm is as follows, ran in a loop until you write enough bytes to fill
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
//!       a time for correct behavior.**
//!     * Copy that amount of bytes from the lookback position to the current position.
//!
//! # Usage
//! This module offers (de)compression in various levels of complexity:
//! ## Decompression
//! * [`decompress_from_path`]: Provide a path and get the decompressed data back
//! * [`decompress_from`]: Load the input data yourself, get decompressed data back
//! * [`decompress_into`]: Load the input and output data yourself, run the decompression
//! ## Compression
//! * [`compress_from_path`]: Provide a path and get the compressed data back
//! * [`compress_from`]: Load the input data yourself, get compressed data back

use core::fmt::Display;
#[cfg(feature = "std")]
use std::path::Path;

use orthrus_core::prelude::*;
use snafu::prelude::*;

/// Error conditions for when reading/writing Yaz0 files
#[derive(Debug, Snafu)]
pub enum Error {
    /// Thrown when trying to open a file or folder that doesn't exist.
    #[snafu(display("Unable to find file/folder!"))]
    NotFound,
    /// Thrown when unable to open a file or folder.
    #[snafu(display("No permissions to open file/folder!"))]
    PermissionDenied,
    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    /// Thrown if yaz0-compressed file is larger than worst possible estimation.
    ///
    /// **This should not be seen in normal use.**
    #[snafu(display("Invalid Size Encountered!"))]
    InvalidSize,
    /// Thrown if the file is larger than u32::MAX since the header cannot store it.
    #[snafu(display("File too large to fit into u32::MAX!"))]
    FileTooBig,
    /// Thrown if the header contains a magic number other than "Yaz0".
    #[snafu(display("Invalid Magic! Expected \"Yaz0\""))]
    InvalidMagic,
}
type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Error::NotFound,
            std::io::ErrorKind::UnexpectedEof => Error::EndOfFile,
            std::io::ErrorKind::PermissionDenied => Error::PermissionDenied,
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

/// Unique identifier that tells us if we're reading a Yaz0-compressed file
pub const MAGIC: [u8; 4] = *b"Yaz0";

/// See the [header](self#header) for more information.
pub struct Yaz0Header {
    pub decompressed_size: u32,
    pub alignment: u32,
}

/// Returns the metadata from a Yaz0 header.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.yaz0")?;
/// let header = yaz0::read_header(&input)?;
/// assert_eq!(header.decompressed_size, 0x40000);
/// assert_eq!(header.alignment, 0);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// Returns [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yaz0 file.
pub fn read_header(data: &[u8]) -> Result<Yaz0Header> {
    let magic = &data[0..4];
    ensure!(magic == MAGIC, InvalidMagicSnafu);

    let decompressed_size = unsafe {
        let ptr = data.as_ptr().add(4);
        u32::from_be_bytes([
            *ptr.offset(0),
            *ptr.offset(1),
            *ptr.offset(2),
            *ptr.offset(3),
        ])
    };

    //0 on GC/Wii files
    let alignment = unsafe {
        let ptr = data.as_ptr().add(8);
        u32::from_be_bytes([
            *ptr.offset(0),
            *ptr.offset(1),
            *ptr.offset(2),
            *ptr.offset(3),
        ])
    };

    Ok(Yaz0Header {
        decompressed_size,
        alignment,
    })
}

/// Loads a Yaz0 file and returns the decompressed data.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let output = yaz0::decompress_from_path("../../examples/assets/tobudx.yaz0")?;
/// assert_eq!(output.len(), 0x40000);
///
/// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// Returns:
/// * [`NotFound`](Error::NotFound) if the path does not exist
/// * [`PermissionDenied`](Error::PermissionDenied) if unable to open the file
/// * [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yaz0 file
/// * [`EndOfFile`](Error::EndOfFile) if trying to read or write out of bounds
#[cfg(feature = "std")]
pub fn decompress_from_path<P: AsRef<Path> + Display>(path: P) -> Result<Box<[u8]>> {
    let input = std::fs::read(path)?;
    self::decompress_from(&input)
}

/// Decompresses a Yaz0 file and returns the decompressed data.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.yaz0")?;
/// let output = yaz0::decompress_from(&input)?;
/// assert_eq!(output.len(), 0x40000);
///
/// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// Returns [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yaz0 file, or
/// [`EndOfFile`](Error::EndOfFile) if trying to read or write out of bounds.
pub fn decompress_from(data: &[u8]) -> Result<Box<[u8]>> {
    let header = read_header(data)?;

    //Allocate decompression buffer
    let mut output = vec![0u8; header.decompressed_size as usize].into_boxed_slice();

    //Perform the actual decompression
    self::decompress_into(data, &mut output)?;

    //If we've gotten this far, output contains valid decompressed data
    Ok(output)
}

/// Decompresses a Yaz0 input file into the output buffer.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.yaz0")?;
/// let header = yaz0::read_header(&input)?;
/// let mut output = vec![0u8; header.decompressed_size as usize].into_boxed_slice();
/// yaz0::decompress_into(&input, &mut output)?;
///
/// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// This function will return [`EndOfFile`](Error::EndOfFile) if trying to read or write out of
/// bounds.
#[inline(never)]
pub fn decompress_into(input: &[u8], output: &mut [u8]) -> Result<()> {
    let mut input_pos: usize = 0x10;
    let mut output_pos: usize = 0x0;
    let mut mask: u8 = 0;
    let mut flags: u8 = 0;

    while output_pos < output.len() {
        //Check if we need a new flag byte
        if mask == 0 {
            ensure!(input.len() > input_pos, EndOfFileSnafu);
            unsafe {
                flags = *input.as_ptr().add(input_pos);
            }
            input_pos += 1;
            mask = 1 << 7;
        }

        //Check what kind of copy we're doing
        if (flags & mask) != 0 {
            //Copy one byte from the input stream
            ensure!(
                (input.len() > input_pos) && (output.len() > output_pos),
                EndOfFileSnafu
            );
            unsafe {
                *output.as_mut_ptr().add(output_pos) = *input.as_ptr().add(input_pos);
            }
            output_pos += 1;
            input_pos += 1;
        } else {
            //RLE copy from previously in the buffer
            ensure!(input.len() >= input_pos + 2, EndOfFileSnafu);
            let code = unsafe {
                let ptr = input.as_ptr().add(input_pos);
                u16::from_be_bytes([*ptr.offset(0), *ptr.offset(1)])
            };
            input_pos += 2;

            //Extract RLE information from the code byte, read another byte for size if we need to
            //How far back in the output buffer do we need to copy from, how many bytes do we copy?
            let back = usize::from((code & 0xFFF) + 1);
            let size = match code >> 12 {
                0 => {
                    ensure!(input.len() > input_pos, EndOfFileSnafu);
                    let value = unsafe { *input.as_ptr().add(input_pos) };
                    input_pos += 1;
                    usize::from(value) + 0x12
                }
                n => usize::from(n) + 2,
            };

            ensure!(
                (output.len() >= output_pos - back + size) && (output.len() >= output_pos + size),
                EndOfFileSnafu
            );
            //If the ranges are not overlapping, use the faster copy method
            if (output_pos - back < output_pos + size) && (output_pos < output_pos - back + size) {
                for n in 0..size {
                    unsafe {
                        *output.as_mut_ptr().add(output_pos + n) =
                            *output.as_ptr().add(output_pos - back + n);
                    }
                }
            } else {
                unsafe {
                    let src_ptr = output.as_ptr().add(output_pos - back);
                    let dest_ptr = output.as_mut_ptr().add(output_pos);
                    core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, size);
                }
            }
            output_pos += size;
        }

        mask >>= 1;
    }

    Ok(())
}

pub enum CompressionAlgo {
    MarioKartWii, //eggCompress
}

/// Loads a Yaz0 file and returns the decompressed data.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let output = yaz0::decompress_from_path("../../examples/assets/tobudx.yaz0")?;
/// assert_eq!(output.len(), 0x40000);
///
/// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// Returns:
/// * [`NotFound`](Error::NotFound) if the path does not exist
/// * [`PermissionDenied`](Error::PermissionDenied) if unable to open the file
/// * [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yaz0 file
/// * [`EndOfFile`](Error::EndOfFile) if trying to read or write out of bounds
#[cfg(feature = "std")]
pub fn compress_from_path<P>(path: P, algo: CompressionAlgo, align: u32) -> Result<Box<[u8]>>
where
    P: AsRef<Path> + Display,
{
    let input = std::fs::read(path)?;
    self::compress_from(&input, algo, align)
}

/// This function compresses the input data using a given compression algorithm.
///
/// # Warnings
/// Alignment should be zero on GameCube and Wii platforms, and non-zero on Wii U and Switch
/// platforms. This function does not try to discern what platform the file is being compressed for,
/// or if the alignment is valid.
pub fn compress_from(input: &[u8], algo: CompressionAlgo, align: u32) -> Result<Box<[u8]>> {
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

    Ok(output.shrink_to(output_size as usize)?.into_inner())
}

fn compress_into_mkw(input: &[u8], output: &mut DataCursor) -> usize {
    let mut input_pos: usize;
    let mut output_pos: usize;
    let mut flag_byte_pos: usize;
    let mut flag_byte_shift: u8;
    let mut flag_byte: u8;

    input_pos = 0;
    output_pos = 0x11;
    flag_byte_pos = 0x10;
    flag_byte_shift = 0x80;
    flag_byte = 0;

    while input_pos < input.len() {
        let mut first_match_offset: usize;
        let mut first_match_len: usize;
        (first_match_offset, first_match_len) = find_match(input, input_pos);
        if first_match_len <= 2 {
            //The longest match we found is less than two bytes, smaller to copy a byte
            flag_byte |= flag_byte_shift;
            output[output_pos] = input[input_pos];
            input_pos += 1;
            output_pos += 1;
        } else {
            let second_match_offset: usize;
            let second_match_len: usize;
            (second_match_offset, second_match_len) = find_match(input, input_pos + 1);
            if first_match_len + 1 < second_match_len {
                flag_byte |= flag_byte_shift;
                flag_byte_shift >>= 1;
                output[output_pos] = input[input_pos];
                input_pos += 1;
                output_pos += 1;
                if flag_byte_shift == 0 {
                    output[flag_byte_pos] = flag_byte;
                    flag_byte = 0;
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
        }

        flag_byte_shift >>= 1;
        if flag_byte_shift == 0 {
            output[flag_byte_pos] = flag_byte;
            flag_byte = 0;
            flag_byte_shift = 0x80;
            flag_byte_pos = output_pos;
            output[output_pos] = 0;
            output_pos += 1;
        }
    }

    if flag_byte != 0 {
        output[flag_byte_pos] = flag_byte;
    }

    output_pos
}

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
    let mut it_haystack: usize;
    let mut it_needle: usize;

    if needle.len() > haystack.len() {
        return haystack.len();
    }
    let skip_table = compute_skip_table(needle);

    it_haystack = needle.len() - 1;
    'outer: loop {
        //SAFETY: needle.len() - 2 will always be within needle, haystack will always be larger,
        //skip will always be bigger than u8::MAX
        unsafe {
            while *haystack.as_ptr().add(it_haystack) != *needle.as_ptr().add(needle.len() - 1) {
                let index = *haystack.as_ptr().add(it_haystack) as usize;
                it_haystack += *skip_table.as_ptr().add(index) as usize;
            }
        }
        it_haystack -= 1;
        it_needle = needle.len() - 2;

        for _ in 0..=it_needle {
            //SAFETY: needle.len() - 2 will always be within needle, haystack will always be
            // larger, skip will always be bigger than u8::MAX
            unsafe {
                let haystack_value = *haystack.as_ptr().add(it_haystack);
                if haystack_value != *needle.as_ptr().add(it_needle) {
                    let mut skip: usize =
                        *skip_table.as_ptr().add(haystack_value as usize) as usize;
                    if needle.len() - it_needle > skip {
                        skip = needle.len() - it_needle;
                    }
                    it_haystack += skip;
                    continue 'outer;
                }
            }
            it_haystack -= 1;
            it_needle -= 1;
        }
        return it_haystack + 1;
    }
}

#[inline(always)]
fn compute_skip_table(needle: &[u8]) -> [u16; 256] {
    let mut table = [needle.len() as u16; 256];
    for i in 0..needle.len() {
        unsafe {
            let index = *needle.as_ptr().add(i) as usize;
            *table.as_mut_ptr().add(index) = (needle.len() - i - 1) as u16;
        }
    }
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
