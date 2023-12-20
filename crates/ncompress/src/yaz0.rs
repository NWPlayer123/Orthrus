//! Adds support for the Yaz0 compression format used for N64, GameCube, Wii, Wii U, and Switch.
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
//! * [`decompress_from_path`]: Provide a path, get decompressed data back
//! * [`decompress_from`]: Provide the input data, get decompressed data back
//! * [`decompress`]: Provide the input data and output buffer, run the decompression algorithm
//! ## Compression
//! * [`compress_from_path`]: Provide a path, get compressed data back
//! * [`compress_from`]: Provide the input data, get compressed data back
//! * [`compress_n64`]: Provide the input data and output buffer, run the compression (older
//!   matching algorithm)

#[cfg(feature = "std")]
use std::{fmt::Display, path::Path};

use snafu::prelude::*;

#[cfg(not(feature = "std"))]
use crate::no_std::*;

/// Error conditions for when reading/writing Yaz0 files
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    /// Thrown when trying to open a file or folder that doesn't exist.
    #[snafu(display("Unable to find file/folder!"))]
    NotFound,
    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    /// Thrown when unable to open a file or folder.
    #[snafu(display("No permissions to open file/folder!"))]
    PermissionDenied,
    /// Thrown if yaz0-compressed file is larger than worst possible estimation.
    ///
    /// **This should not be seen in normal use.**
    #[snafu(display("Invalid Size Encountered!"))]
    InvalidSize,
    /// Thrown if the file is larger than u32::MAX since the header cannot store it.
    #[snafu(display("File too large to fit into u32::MAX!"))]
    FileTooBig,
    /// Thrown if the header contains a magic number other than "Yaz0".
    #[snafu(display("Invalid Magic! Expected {:?}.", MAGIC))]
    InvalidMagic,
}
type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::UnexpectedEof => Self::EndOfFile,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => panic!("Unexpected std::io::error! Something has gone horribly wrong"),
        }
    }
}

/// Unique identifier that tells us if we're reading a Yaz0-compressed file
pub const MAGIC: [u8; 4] = *b"Yaz0";

/// See the module [header](self#header) for more information.
pub struct Header {
    pub decompressed_size: u32,
    pub alignment: u32,
}

/// Returns the metadata from a Yaz0 header.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.yaz0_n64")?;
/// let header = yaz0::read_header(&input)?;
/// assert_eq!(header.decompressed_size, 0x40000);
/// assert_eq!(header.alignment, 0);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// Returns [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yaz0 file.
#[inline]
pub fn read_header(data: &[u8]) -> Result<Header> {
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

    Ok(Header {
        decompressed_size,
        alignment,
    })
}

/// Loads a Yaz0 file and returns the decompressed data.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let output = yaz0::decompress_from_path("../../examples/assets/tobudx.yaz0_n64")?;
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
#[inline]
pub fn decompress_from_path<P: AsRef<Path> + Display>(path: P) -> Result<Box<[u8]>> {
    let input = std::fs::read(path)?;
    self::decompress_from(&input)
}

/// Decompresses a Yaz0 file and returns the decompressed data.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.yaz0_n64")?;
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
#[inline]
pub fn decompress_from(data: &[u8]) -> Result<Box<[u8]>> {
    let header = read_header(data)?;

    //Allocate decompression buffer
    let mut output = vec![0u8; header.decompressed_size as usize].into_boxed_slice();

    //Perform the actual decompression
    self::decompress(data, &mut output)?;

    //If we've gotten this far, output contains valid decompressed data
    Ok(output)
}

/// Decompresses a Yaz0 input file into the output buffer.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.yaz0_n64")?;
/// let header = yaz0::read_header(&input)?;
/// let mut output = vec![0u8; header.decompressed_size as usize].into_boxed_slice();
/// yaz0::decompress(&input, &mut output)?;
///
/// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// This function will return [`EndOfFile`](Error::EndOfFile) if trying to read or write out of
/// bounds.
#[inline]
pub fn decompress(input: &[u8], output: &mut [u8]) -> Result<()> {
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
            output[output_pos] = input[input_pos];
            output_pos += 1;
            input_pos += 1;
        } else {
            //RLE copy from previously in the buffer
            let code = u16::from_be_bytes([input[input_pos], input[input_pos + 1]]);
            input_pos += 2;

            //Extract RLE information from the code byte, read another byte for size if we need to
            //How far back in the output buffer do we need to copy from, how many bytes do we copy?
            let back = usize::from((code & 0xFFF) + 1);
            let size = match code >> 12 {
                0 => {
                    let value = input[input_pos];
                    input_pos += 1;
                    usize::from(value) + 0x12
                }
                n => usize::from(n) + 2,
            };

            //If the ranges are not overlapping, use the faster copy method
            if (output_pos - back < output_pos + size) && (output_pos < output_pos - back + size) {
                for n in 0..size {
                    output[output_pos + n] = output[output_pos - back + n];
                }
            } else {
                output.copy_within(output_pos - back..output_pos - back + size, output_pos);
            }
            output_pos += size;
        }

        mask >>= 1;
    }

    Ok(())
}

/// All supported Yaz0 compression algorithms
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum CompressionAlgo {
    MatchingOld, //eggCompress
    //MatchingNew, //MK8
}

/// Loads a Yaz0 file and returns the compressed data.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let output = yaz0::compress_from_path(
///     "../../examples/assets/tobudx.gb",
///     yaz0::CompressionAlgo::MatchingOld,
///     0,
/// )?;
///
/// let expected = std::fs::read("../../examples/assets/tobudx.yaz0_n64")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Errors
/// Returns [`FileTooBig`](Error::FileTooBig) if the input is too large for the filesize to be
/// stored in the header.
#[cfg(feature = "std")]
#[inline]
pub fn compress_from_path<P>(path: P, algo: CompressionAlgo, align: u32) -> Result<Box<[u8]>>
where
    P: AsRef<Path> + Display,
{
    let input = std::fs::read(path)?;
    self::compress_from(&input, algo, align)
}

/// Calculates the filesize for the largest possible file that can be created with Yaz0 compression.
///
/// This consists of the 0x10 header, the length of the input file, and all flag bits needed,
/// rounded up.
#[must_use]
#[inline]
pub const fn worst_possible_size(input_len: usize) -> usize {
    0x10 + input_len + input_len.div_ceil(8)
}

/// Compresses the input data using a given compression algorithm.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.gb")?;
/// let output = yaz0::compress_from(&input, yaz0::CompressionAlgo::MatchingOld, 0)?;
///
/// let expected = std::fs::read("../../examples/assets/tobudx.yaz0_n64")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
///
/// # Warnings
/// Alignment should be zero for N64, GameCube, and Wii, and should be non-zero on Wii U and Switch.
///
/// # Errors
/// Returns [`FileTooBig`](Error::FileTooBig) if the input is too large for the filesize to be
/// stored in the header.
#[inline]
pub fn compress_from(input: &[u8], algo: CompressionAlgo, _align: u32) -> Result<Box<[u8]>> {
    ensure!(u32::try_from(input.len()).is_ok(), FileTooBigSnafu);

    //Assume 0x10 header, every byte is a copy, and include flag bytes (rounded up)
    let mut output = vec![0u8; worst_possible_size(input.len())];

    let output_size = match algo {
        CompressionAlgo::MatchingOld => compress_n64(input, &mut output),
    };

    output.truncate(output_size);

    Ok(output.into_boxed_slice())
}

/// Compresses the input using Nintendo's pre-Wii U algorithm, and returns the size of the
/// compressed data.
///
/// This algorithm should create identically compressed files to those from N64, GameCube, and Wii
/// Nintendo games. It does not allow for setting the alignment, as theoretically no files created
/// using this algorithm should have a header with alignment.
///
/// # Examples
/// ```
/// # use orthrus_ncompress::yaz0;
/// let input = std::fs::read("../../examples/assets/tobudx.gb")?;
/// let mut output = vec![0u8; yaz0::worst_possible_size(input.len())];
/// let output_size = yaz0::compress_n64(&input, &mut output);
/// output.truncate(output_size);
///
/// let expected = std::fs::read("../../examples/assets/tobudx.yaz0_n64")?;
/// assert_eq!(*output, *expected);
/// # Ok::<(), yaz0::Error>(())
/// ```
#[inline]
pub fn compress_n64(input: &[u8], output: &mut [u8]) -> usize {
    output[0..4].copy_from_slice(b"Yaz0");
    output[4..8].copy_from_slice(&u32::to_be_bytes(input.len() as u32));
    //Older files do not have alignment so this just leaves it as zero

    let mut input_pos = 0;
    let mut output_pos = 0x11;
    let mut flag_byte_pos = 0x10;
    let mut flag_byte_shift = 0x80;

    while input_pos < input.len() {
        let (mut group_offset, mut group_size) = find_match(input, input_pos);
        if group_size <= 2 {
            //If the group is less than two bytes, it's smaller to just copy a byte
            output[flag_byte_pos] |= flag_byte_shift;
            output[output_pos] = input[input_pos];
            input_pos += 1;
            output_pos += 1;
        } else {
            //Check one byte after this, see if we can get a better match
            let (new_offset, new_size) = find_match(input, input_pos + 1);
            if group_size + 1 < new_size {
                //If we did find a better match, copy a byte and then use the new slice
                output[flag_byte_pos] |= flag_byte_shift;
                output[output_pos] = input[input_pos];
                input_pos += 1;
                output_pos += 1;

                //Check if we need to create a new flag byte
                flag_byte_shift >>= 1;
                if flag_byte_shift == 0 {
                    flag_byte_shift = 0x80;
                    flag_byte_pos = output_pos;
                    output[output_pos] = 0;
                    output_pos += 1;
                }

                //Use the new slice for the lookback data
                group_size = new_size;
                group_offset = new_offset;
            }

            //Calculate the lookback offset
            group_offset = input_pos - group_offset - 1;

            //If we can't fit the size in the upper nibble, write a third byte for the length
            if group_size >= 0x12 {
                output[output_pos] = (group_offset >> 8) as u8;
                output[output_pos + 1] = (group_offset) as u8;
                output[output_pos + 2] = (group_size - 0x12) as u8;
                output_pos += 3;
            } else {
                output[output_pos] = (((group_size - 2) << 4) | (group_offset >> 8)) as u8;
                output[output_pos + 1] = (group_offset) as u8;
                output_pos += 2;
            }
            input_pos += group_size;
        }

        //Check if we need to create a new flag byte
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

/// Maximum distance to look back in the buffer for a match (0xFFF for lower 3 nibbles + 1)
const MAX_LOOKBACK: usize = 0x1000;
/// Maximum number of bytes that can be copied from the lookback (0x12 threshold for a third byte +
/// 0xFF from that byte)
const MAX_COPY_SIZE: usize = 0x111;

/// Finds the biggest match in the lookback window for the bytes at the current input position.
fn find_match(input: &[u8], input_pos: usize) -> (usize, usize) {
    //Setup the initial location and size for the lookback window
    let mut window = core::cmp::max(input_pos.saturating_sub(MAX_LOOKBACK), 0);
    let mut window_size = 3;

    //This is the maximum we're able to copy in a single operation
    let max_match_size = core::cmp::min(input.len().saturating_sub(input_pos), MAX_COPY_SIZE);

    //If we can't copy more than two bytes (the size of the copy data) then don't bother looking
    if max_match_size < 3 {
        return (0, 0);
    }

    let mut window_offset = 0;
    let mut found_match_offset = 0;

    //Look for a match while we're within the range of the lookback buffer
    while window < input_pos && {
        window_offset = search_window(
            &input[input_pos..input_pos + window_size],
            &input[window..input_pos + window_size],
        );
        window_offset < input_pos - window
    } {
        //Expand the needle as long as it still matches the spot we found in the haystack
        while window_size < max_match_size
            && input[window + window_offset + window_size] == input[input_pos + window_size]
        {
            window_size += 1;
        }

        //If we've hit the max match size, we can't find a bigger match so just return it
        if window_size == max_match_size {
            return (window + window_offset, max_match_size);
        }

        found_match_offset = window + window_offset;
        window += window_offset + 1;
        window_size += 1;
    }

    //Return the biggest match we found, potentially none
    (
        found_match_offset,
        if window_size > 3 { window_size - 1 } else { 0 },
    )
}

/// Searches for the needle in the haystack using a modified version of Horspool's algorithm, and
/// returns the index of the first match
#[inline]
fn search_window(needle: &[u8], haystack: &[u8]) -> usize {
    //Check if we can even find the needle
    if needle.len() > haystack.len() {
        return haystack.len();
    }

    //Calculate the skip table for searching for end characters
    let skip_table = compute_skip_table(needle);

    let mut haystack_index = needle.len() - 1;
    'outer: loop {
        //Loop while we look for the last character of the needle, skipping through the haystack
        while haystack[haystack_index] != needle[needle.len() - 1] {
            haystack_index += skip_table[haystack[haystack_index] as usize] as usize;
        }
        haystack_index -= 1;

        //Found a possible match with the end character, now check if the rest of the needle
        // matches
        for needle_index in (0..needle.len() - 1).rev() {
            //If it doesn't, skip ahead and go back to searching for another end character
            if haystack[haystack_index] != needle[needle_index] {
                let mut skip: usize = skip_table[haystack[haystack_index] as usize] as usize;

                if needle.len() - needle_index > skip {
                    skip = needle.len() - needle_index;
                }
                haystack_index += skip;
                continue 'outer;
            }
            haystack_index = haystack_index.wrapping_sub(1);
        }
        return haystack_index.wrapping_add(1);
    }
}

/// Creates the skip table for Horspool's algorithm which contains how much farther to look forward
/// in the haystack in order for there to possibly be a match for the needle.
#[inline(always)]
fn compute_skip_table(needle: &[u8]) -> [u16; 256] {
    let mut table = [needle.len() as u16; 256];

    for i in 0..needle.len() {
        table[needle[i] as usize] = (needle.len() - i - 1) as u16;
    }

    table
}
