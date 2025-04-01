//! Adds support for the Yay0 compression format used for first-party N64 and early GameCube games.
//!
//! Because the Yay0 format is so lightweight, this module is designed to not have any persistence.
//! It takes in data, and will return the de/compressed data contained inside.
//!
//! # Format
//! The Yay0 format is part of the [Lempel-Ziv family of algorithms](https://w.wiki/F6n), which use
//! a "sliding window" to allow for copying repetitive data from previously in the output buffer.
//! The input is split into three sections. The first contains flag bytes that signal whether to
//! copy from the input or from the output buffer, the second contains all copyable data, and the
//! third contains all lookback+length pairs.
//!
//! ## Header
//! The header is as follows, in big-endian format:
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | 0x0 | Magic number     | u8\[4] | Unique identifier ("Yay0") to let us know we're reading a Yay0-compressed file. |
//! | 0x4 | Output size      | u32    | The size of the decompressed data, needed for the output buffer. |
//! | 0x8 | Copy data offset | u32    | Offset to the copyable data section. See [format](self#format) for details. |
//! | 0xC | Lookback offset  | u32    | Offset to the lookback data section. See [format](self#format) for details. |
//!
//! # Decompression
//! The decompression algorithm is as follows, ran in a loop until you write enough bytes to fill
//! the output buffer:
//!
//! * Set three pointers, one to header+0x10 for flag data, one to the copy data offset, and one to the
//!   lookback offset.
//! * Read one byte from the flag data, which is 8 flag bits from high to low.
//! * For each flag bit, if it is a 1, copy one byte from the data section to the output.
//! * If it is a 0, copy bytes from earlier in the output buffer:
//!     * Read two bytes from the lookback section.
//!     * Get the first nibble (code >> 12). If it is 0, read one byte ***from the copy data section*** and
//!       add 18 (0x12). Otherwise, add 2 to the nibble. Use that as the number of bytes to copy.
//!     * Add 1 to the lower nibbles (code & 0xFFF) and treat that as how far back in the buffer to read, from
//!       the current position.
//!     * **Note that the count can overlap with the destination, and needs to be copied one byte at a time
//!       for correct behavior.**
//!     * Copy that amount of bytes from the lookback position to the current position.
//!
//! # Usage
//! This module offers the following functionality:
//! ## Decompression
//! * [`decompress_from_path`](Yay0::decompress_from_path): Provide a path, get decompressed data back
//! * [`decompress_from`](Yay0::decompress_from): Provide the input data, get decompressed data back
//! * [`decompress`](Yay0::decompress): Provide the input data and output buffer, run the decompression
//!   algorithm
//! ## Compression
//! * [`compress_from_path`](Yay0::compress_from_path): Provide a path, get compressed data back
//! * [`compress_from`](Yay0::compress_from): Provide the input data, get compressed data back
//! * [`compress_n64`](Yay0::compress_n64): Provide the input data and output buffer, run the compression
//!   (matching algorithm)
//! ## Utilities
//! * [`read_header`](Yay0::read_header): Returns the header information for a given Yay0 file
//! * [`worst_possible_size`](Yay0::worst_possible_size): Calculates the worst possible compression size for a
//!   given filesize

#[cfg(feature = "std")]
use std::path::Path;

use orthrus_core::prelude::*;
use snafu::prelude::*;

#[cfg(not(feature = "std"))]
use crate::no_std::*;

/// Error conditions for when reading/writing Yay0 files
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
    /// Thrown if Yay0-compressed file is larger than worst possible estimation.
    ///
    /// **This should not be seen in normal use.**
    #[snafu(display("Invalid Size Encountered!"))]
    InvalidSize,
    /// Thrown if the file is larger than u32::MAX since the header cannot store it.
    #[snafu(display("File too large to fit into u32::MAX!"))]
    FileTooBig,
    /// Thrown if the header contains a magic number other than "Yay0".
    #[snafu(display("Invalid Magic! Expected {:?}.", Yay0::MAGIC))]
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

/// See the module [header](self#header) for more information.
pub struct Header {
    pub decompressed_size: u32,
    pub lookback_offset: u32,
    pub copy_data_offset: u32,
}

/// All supported Yay0 compression algorithms
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum CompressionAlgo {
    /// This algorithm should create identical files for all data from N64 and GameCube.
    MatchingOld, //eggCompress
}

/// Utility struct for handling Yay0 compression.
///
/// Yay0 is stateless, and is merely a namespace for implementing certain traits.
///
/// See the [module documentation](self) for more information.
pub struct Yay0;

impl Yay0 {
    /// Unique identifier that tells us if we're reading a Yay0-compressed file
    pub const MAGIC: [u8; 4] = *b"Yay0";

    /// Returns the metadata from a Yay0 header.
    ///
    /// # Examples
    /// ```
    /// # use orthrus_ncompress::prelude::*;
    /// let input = std::fs::read("../../examples/assets/tobudx.yay0_n64")?;
    /// let header = Yay0::read_header(&input)?;
    /// assert_eq!(header.decompressed_size, 0x40000);
    /// assert_eq!(header.lookback_offset, 0x274C);
    /// assert_eq!(header.copy_data_offset, 0xD0D4);
    /// # Ok::<(), yay0::Error>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yay0 file.
    #[inline]
    pub fn read_header(data: &[u8]) -> Result<Header> {
        // Make sure we have enough data to actually check a header
        ensure!(data.len() <= 0x10, EndOfFileSnafu);

        let magic = &data[0..4];
        ensure!(magic == Self::MAGIC, InvalidMagicSnafu);

        let decompressed_size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let lookback_offset = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let copy_data_offset = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);

        Ok(Header { decompressed_size, lookback_offset, copy_data_offset })
    }

    /// Calculates the filesize for the largest possible file that can be created with Yay0
    /// compression.
    ///
    /// This consists of the 0x10 header, the length of the input file, and all flag bits needed,
    /// rounded up, with all sections aligned to 4 bytes and the file aligned to a 0x10 byte
    /// boundary.
    #[must_use]
    #[inline]
    pub const fn worst_possible_size(input_len: usize) -> usize {
        let full_size = 0x10 + ((input_len + 3) & !3) + ((input_len.div_ceil(8) + 3) & !3);
        (full_size + 15) & !15
    }

    /// Loads a Yay0 file and returns the decompressed data.
    ///
    /// # Examples
    /// ```
    /// # use orthrus_ncompress::prelude::*;
    /// let output = Yay0::decompress_from_path("../../examples/assets/tobudx.yay0_n64")?;
    /// assert_eq!(output.len(), 0x40000);
    ///
    /// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
    /// assert_eq!(*output, *expected);
    /// # Ok::<(), yay0::Error>(())
    /// ```
    ///
    /// # Errors
    /// Returns:
    /// * [`NotFound`](Error::NotFound) if the path does not exist
    /// * [`PermissionDenied`](Error::PermissionDenied) if unable to open the file
    /// * [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yay0 file
    #[cfg(feature = "std")]
    #[inline]
    pub fn decompress_from_path<P: AsRef<Path>>(path: P) -> Result<Box<[u8]>> {
        let input = std::fs::read(path)?;
        Self::decompress_from(&input)
    }

    /// Decompresses a Yay0 file and returns the decompressed data.
    ///
    /// # Examples
    /// ```
    /// # use orthrus_ncompress::prelude::*;
    /// let input = std::fs::read("../../examples/assets/tobudx.yay0_n64")?;
    /// let output = Yay0::decompress_from(&input)?;
    /// assert_eq!(output.len(), 0x40000);
    ///
    /// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
    /// assert_eq!(*output, *expected);
    /// # Ok::<(), yay0::Error>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the header does not match a Yay0 file.
    #[inline]
    pub fn decompress_from(data: &[u8]) -> Result<Box<[u8]>> {
        let header = Self::read_header(data)?;

        //Allocate decompression buffer
        let mut output = vec![0u8; header.decompressed_size as usize].into_boxed_slice();

        //Perform the actual decompression
        Self::decompress(data, &mut output, header.lookback_offset, header.copy_data_offset);

        //If we've gotten this far, output contains valid decompressed data
        Ok(output)
    }

    /// Decompresses a Yay0 input file into the output buffer.
    ///
    /// # Examples
    /// ```
    /// # use orthrus_ncompress::prelude::*;
    /// let input = std::fs::read("../../examples/assets/tobudx.yay0_n64")?;
    /// let header = Yay0::read_header(&input)?;
    /// let mut output = vec![0u8; header.decompressed_size as usize];
    /// Yay0::decompress(
    ///     &input,
    ///     &mut output,
    ///     header.lookback_offset,
    ///     header.copy_data_offset,
    /// );
    ///
    /// let expected = std::fs::read("../../examples/assets/tobudx.gb")?;
    /// assert_eq!(*output, *expected);
    /// # Ok::<(), yay0::Error>(())
    /// ```
    #[inline]
    pub fn decompress(input: &[u8], output: &mut [u8], lookback: u32, copy_data: u32) {
        //Setup all three offsets
        let mut flag_offset: usize = 0x10;
        let mut lookback_offset: usize = lookback as usize;
        let mut copy_data_offset: usize = copy_data as usize;

        let mut output_pos: usize = 0x0;
        let mut mask: u8 = 0;
        let mut flags: u8 = 0;

        while output_pos < output.len() {
            //Check if we need a new flag byte
            if mask == 0 {
                flags = input[flag_offset];
                flag_offset += 1;
                mask = 1 << 7;
            }

            //Check what kind of copy we're doing
            if (flags & mask) != 0 {
                //Copy one byte from the input stream
                output[output_pos] = input[copy_data_offset];
                copy_data_offset += 1;
                output_pos += 1;
            } else {
                //RLE copy from previously in the buffer
                let code = u16::from_be_bytes([input[lookback_offset], input[lookback_offset + 1]]);
                lookback_offset += 2;

                //Extract RLE information from the code byte, read another byte for size if we need
                // to How far back in the output buffer do we need to copy from, how
                // many bytes do we copy?
                let back = output_pos - usize::from((code & 0xFFF) + 1);
                let size = match code >> 12 {
                    0 => {
                        let value = input[copy_data_offset];
                        copy_data_offset += 1;
                        usize::from(value) + 0x12
                    }
                    n => usize::from(n) + 2,
                };

                //If the ranges are not overlapping, use the faster copy method
                if (back < output_pos + size) && (output_pos < back + size) {
                    for n in 0..size {
                        output[output_pos + n] = output[back + n];
                    }
                } else {
                    output.copy_within(back..back + size, output_pos);
                }
                output_pos += size;
            }

            mask >>= 1;
        }
    }

    /// Loads a Yay0 file and returns the compressed data.
    ///
    /// # Examples
    /// ```
    /// # use orthrus_ncompress::prelude::*;
    /// let output = Yay0::compress_from_path(
    ///     "../../examples/assets/tobudx.gb",
    ///     yay0::CompressionAlgo::MatchingOld,
    ///     0,
    /// )?;
    ///
    /// let expected = std::fs::read("../../examples/assets/tobudx.yay0_n64")?;
    /// assert_eq!(*output, *expected);
    /// # Ok::<(), yay0::Error>(())
    /// ```
    ///
    /// # Errors
    /// Returns:
    /// * [`NotFound`](Error::NotFound) if the path does not exist
    /// * [`PermissionDenied`](Error::PermissionDenied) if unable to open the file
    /// * [`FileTooBig`](Error::FileTooBig) if too large for the filesize to be stored in the header
    #[cfg(feature = "std")]
    #[inline]
    pub fn compress_from_path<P>(path: P, algo: CompressionAlgo, align: u32) -> Result<Box<[u8]>>
    where
        P: AsRef<Path>,
    {
        let input = std::fs::read(path)?;
        Self::compress_from(&input, algo, align)
    }

    /// Compresses the input data using a given compression algorithm.
    ///
    /// # Examples
    /// ```
    /// # use orthrus_ncompress::prelude::*;
    /// let input = std::fs::read("../../examples/assets/tobudx.gb")?;
    /// let output = Yay0::compress_from(&input, yay0::CompressionAlgo::MatchingOld, 0)?;
    ///
    /// let expected = std::fs::read("../../examples/assets/tobudx.yay0_n64")?;
    /// assert_eq!(*output, *expected);
    /// # Ok::<(), yay0::Error>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`FileTooBig`](Error::FileTooBig) if the input is too large for the filesize to be
    /// stored in the header.
    #[inline]
    pub fn compress_from(input: &[u8], algo: CompressionAlgo, _align: u32) -> Result<Box<[u8]>> {
        ensure!(u32::try_from(input.len()).is_ok(), FileTooBigSnafu);

        //Assume 0x10 header, every byte is a copy, and include flag bytes (rounded up)
        let mut output = vec![0u8; Self::worst_possible_size(input.len())];

        let output_size = match algo {
            CompressionAlgo::MatchingOld => Self::compress_n64(input, &mut output),
        };

        output.truncate(output_size);

        Ok(output.into_boxed_slice())
    }

    /// Compresses the input using Nintendo's pre-Wii U algorithm, and returns the size of the
    /// compressed data.
    ///
    /// This algorithm should create identically compressed files to those from first-party N64 and
    /// GameCube games.
    ///
    /// # Examples
    /// ```
    /// # use orthrus_ncompress::prelude::*;
    /// let input = std::fs::read("../../examples/assets/tobudx.gb")?;
    /// let mut output = vec![0u8; Yay0::worst_possible_size(input.len())];
    /// let output_size = Yay0::compress_n64(&input, &mut output);
    /// output.truncate(output_size);
    ///
    /// let expected = std::fs::read("../../examples/assets/tobudx.yay0_n64")?;
    /// assert_eq!(*output, *expected);
    /// # Ok::<(), yay0::Error>(())
    /// ```
    #[inline]
    pub fn compress_n64(input: &[u8], output: &mut [u8]) -> usize {
        //Set up all arrays so we can accumulate data before writing it, since we don't know how
        // big each section can be
        let mut flag_data = vec![0u8; input.len().div_ceil(8)];
        let mut flag_byte = 0;
        let mut flag_shift = 0x80;
        let mut flag_pos = 0;
        let mut copy_data = vec![0u8; input.len()];
        let mut copy_pos = 0;
        //We only consider writing lookback if it's two bytes or more, so maximum will be two bytes
        // = two bytes aka input.len()
        let mut lookback_data = vec![0u8; input.len()];
        let mut lookback_pos = 0;

        let mut window = crate::algorithms::Window::new(input, 0x111);

        let mut input_pos = 0;

        while input_pos < input.len() {
            let (mut group_offset, mut group_size) = window.search(input_pos);
            if group_size <= 2 {
                //If the group is less than two bytes, it's smaller to just copy a byte
                flag_byte |= flag_shift;
                copy_data[copy_pos] = input[input_pos];
                input_pos += 1;
                copy_pos += 1;
            } else {
                //Check one byte after this, see if we can get a better match
                let (new_offset, new_size) = window.search(input_pos + 1);
                if group_size + 1 < new_size {
                    //If we did find a better match, copy a byte and then use the new slice
                    flag_byte |= flag_shift;
                    copy_data[copy_pos] = input[input_pos];
                    input_pos += 1;
                    copy_pos += 1;

                    //Check if we need to create a new flag byte
                    flag_shift >>= 1;
                    if flag_shift == 0 {
                        flag_shift = 0x80;
                        flag_data[flag_pos] = flag_byte;
                        flag_byte = 0;
                        flag_pos += 1;
                    }

                    //Use the new slice for the lookback data
                    group_size = new_size;
                    group_offset = new_offset;
                }

                //Calculate the lookback offset
                group_offset = input_pos as u32 - group_offset - 1;

                //If we can't fit the size in the upper nibble, write a third byte for the length
                if group_size >= 0x12 {
                    lookback_data[lookback_pos] = (group_offset >> 8) as u8;
                    lookback_data[lookback_pos + 1] = group_offset as u8;
                    lookback_pos += 2;

                    copy_data[copy_pos] = (group_size - 0x12) as u8;
                    copy_pos += 1;
                } else {
                    lookback_data[lookback_pos] = (((group_size - 2) << 4) | (group_offset >> 8)) as u8;
                    lookback_data[lookback_pos + 1] = (group_offset) as u8;
                    lookback_pos += 2;
                }
                input_pos += group_size as usize;
            }

            //Check if we need to create a new flag byte
            flag_shift >>= 1;
            if flag_shift == 0 {
                flag_shift = 0x80;
                flag_data[flag_pos] = flag_byte;
                flag_byte = 0;
                flag_pos += 1;
            }
        }

        //Check if there's still data to flush
        if flag_byte != 0 {
            flag_data[flag_pos] = flag_byte;
            flag_pos += 1;
        }

        //Now we can write the header and flush out our data
        let mut output_pos: usize = 0x10;
        output[0..4].copy_from_slice(b"Yay0");
        output[4..8].copy_from_slice(&u32::to_be_bytes(input.len() as u32));
        output[0x10..0x10 + flag_pos].copy_from_slice(&flag_data[..flag_pos]);
        output_pos += (flag_pos + 3) & !3;
        output[8..12].copy_from_slice(&u32::to_be_bytes(output_pos as u32));
        output[output_pos..output_pos + lookback_pos].copy_from_slice(&lookback_data[..lookback_pos]);
        output_pos += (lookback_pos + 3) & !3;
        output[12..16].copy_from_slice(&u32::to_be_bytes(output_pos as u32));
        output[output_pos..output_pos + copy_pos].copy_from_slice(&copy_data[..copy_pos]);
        output_pos += (copy_pos + 3) & !3;

        (output_pos + 15) & !15
    }
}

impl FileIdentifier for Yay0 {
    fn identify(data: &[u8]) -> Option<FileInfo> {
        Self::read_header(data).ok().map(|header| {
            let info = format!(
                "Nintendo Yay0-compressed file, decompressed size: {}",
                util::format_size(header.decompressed_size as usize)
            );
            FileInfo::new(info, None)
        })
    }

    fn identify_deep(data: &[u8]) -> Option<FileInfo> {
        Self::read_header(data).ok().map(|header| {
            let info = format!(
                "Nintendo Yay0-compressed file, decompressed size: {}",
                util::format_size(header.decompressed_size as usize)
            );
            let payload = Self::decompress_from(data).ok();
            FileInfo::new(info, payload)
        })
    }
}
