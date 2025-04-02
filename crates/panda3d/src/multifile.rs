//! Adds support for the Multifile archive format used by the Panda3D engine.
//!
//! This module is designed to support both "standalone" or one-shot operations that don't require
//! you to keep track of an object, and holding a Multifile in-memory for more complex operations.
//!
//! # Revisions
//! * **Version 1.0**: Initial Multifile Support
//! * **Version 1.1**: Added support for timestamps both for the Multifile as a whole, and individual
//!   Subfiles. Subfiles with a timestamp of zero will use the Multifile timestamp.
//!
//! # Format
//! The Multifile format is designed as a header, and then a number of [`Subfile`]s connected via a
//! linked list structure.
//!
//! It has primitive support for larger-than-32-bit filesizes, using a "scale factor" that
//! multiplies all offsets by a specific amount, which introduces extra padding to meet alignment
//! requirements.
//!
//! It supports comment lines before the Multifile data using a '#' at the start which allows a
//! Multifile to be ran directly from the command line on Unix systems.
//!
//! ## Multifile Header
//! The header is as follows, in little-endian format:
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | 0x0    | Magic number   | u8\[6] | Unique identifier ("pmf\\0\\n\\r") to let us know we're reading a Multifile. |
//! | 0x6    | Major version  | u16    | Major version number for the Multifile revision (currently 1). |
//! | 0x8    | Minor version  | u16    | Minor version number for the Multifile revision (currently 1). |
//! | 0xA    | Scale factor   | u32    | Allows for scaling all file offsets to support larger-than-4GB files. |
//! | \[0xE] | Unix timestamp | u32    | The time the Multifile was last modified ***(revision 1.1+)***. |
//!
//! ## Subfile Header
//!
//! Directly following the Multifile header is a Subfile header. The first "entry" of each header is
//! an offset to the next header, which forms a linked list, with the last entry being a 0. All
//! offsets are relative to the file start, and must be multiplied by the scale factor.
//!
//! *Note that the linked list is not treated as part of the Subfile header, it is only for
//! navigating the Multifile, and is only included here for convenience.*
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | -0x4    | Index offset    | u32        | Offset to the next header, relative to file start. ***Not part of the Subfile.*** |
//! | 0x0     | Data offset     | u32        | Offset to the Subfile's data, relative to file start. |
//! | 0x4     | Data length     | u32        | Length of the Subfile's data. |
//! | 0x8     | File attributes | u16        | Bit-field of the Subfile's attributes, such as [compression or encryption](#subfile-flags). |
//! | \[0xA]  | Original length | u32        | Length of the processed data. ***Only if [compressed or encrypted](#subfile-flags)***. |
//! | \[0xE]  | Unix timestamp  | u32        | The time the Subfile was last modified ***(revision 1.1+)***. If zero, use Multifile timestamp. |
//! | 0x12    | Filename length | u16        | Length of the Subfile's name in the next field. |
//! | 0x14    | Subfile path    | char\[len] | Path to the Subfile, for use in a Virtual File System. Obfuscated, convert as (255 - x). |
//!
//!
//! ## Subfile Flags
//! | Flags | Value | Notes |
//! |-------|-------|-------|
//! | `Deleted`      | 1 << 0 (1)  | The Subfile has been "deleted" from the Multifile, and should be ignored. |
//! | `IndexInvalid` | 1 << 1 (2)  | The Subfile has a corrupt index entry. |
//! | `DataInvalid`  | 1 << 2 (4)  | The Subfile has invalid data associated. |
//! | `Compressed`   | 1 << 3 (8)  | The Subfile is compressed. |
//! | `Encrypted`    | 1 << 4 (16) | The Subfile is encrypted. |
//! | `Signature`    | 1 << 5 (32) | The Subfile is the certificate used to sign the Multifile. See [Certificate Format](#certificate-format) for details. |
//! | `Text`         | 1 << 6 (64) | The Subfile contains text instead of binary data. |
//!
//! ## Certificate Format
//! The Multifile certificate is a binary blob that can contain multiple certificates in a
//! certificate chain, along with the actual signature of the Multifile.
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | 0x0     | Signature length  | u32        | Length of the signature that follows. |
//! | 0x4     | Signature         | char\[len] | Signature used to verify that the Multifile hasn't been altered. |
//! | 0x4+len | Certificate count | u32        | Number of certificates in the data blob that follows. |
//! | 0x8+len | Certificate chain | char\[?]   | Certificate chain used to sign the Multifile. Unknown length, must be parsed*. |
//!
//! * Note: since there is no associated length with the certificate blob, it must be parsed using a
//!   certificate library that returns the "remaining" data, which can then be parsed again, until you extract
//!   (count) certificates. See `d2i_X509` in OpenSSL.
//!
//! # Usage
//! This module can be used either with borrowed data or as an in-memory archive.
//!
//! ## Stateful Functions
//! A Multifile can be created through [`open`](Multifile::open), which will read a file from
//! disk, and [`load`](Multifile::load), which will read the provided file.
//!
//! Once created, the following functions can be used to manipulate the archive:
//!
//! * [`extract_all`](Multifile::extract_all): Save all contained [`Subfile`]s to a given folder
//!
//! ## Stateless Functions
//! These functions can be used without having to first create a Multifile, used for the
//! following one-shot operations:
//!
//! * [`extract_from_path`](Multifile::extract_from_path): Reads a Multifile from disk, and saves all
//!   [`Subfile`]s to a given folder
//! * [`extract_from`](Multifile::extract_from): Reads the provided Multifile, and saves all [`Subfile`]s to a
//!   given folder

#[cfg(feature = "std")]
use std::path::Path;

use orthrus_core::prelude::*;
use snafu::prelude::*;

#[cfg(not(feature = "std"))]
use crate::no_std::*;
use crate::{common::Version, subfile::*};

/// Error conditions for when working with Multifile archives.
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
    /// Thrown if the header contains a magic number other than "pmf\0\n\r".
    #[snafu(display("Invalid Magic! Expected {:?}.", Multifile::MAGIC))]
    InvalidMagic,
    /// Thrown if the header version is too new to be supported.
    #[snafu(display("Unknown Multifile Version! Expected >= v{}.", Multifile::CURRENT_VERSION))]
    UnknownVersion,
}
pub(crate) type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::UnexpectedEof => Self::EndOfFile,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            kind => {
                panic!("Unexpected std::io::error: {kind}! Something has gone horribly wrong")
            }
        }
    }
}

impl From<DataError> for Error {
    #[inline]
    fn from(error: DataError) -> Self {
        match error {
            DataError::EndOfFile => Self::EndOfFile,
            _ => panic!("Unexpected data::error! Something has gone horribly wrong"),
        }
    }
}

struct Header {
    version: Version,
    scale_factor: u32,
    timestamp: u32,
}

// The current least terrible way to implement state in this system is to just store the entire
// Multifile's data, and have each Subfile keep an offset+length. In the future, once safe transmute
// is a thing, I can "take" each header and what's left will be all the relevant file data.

/// Used for working with Multifile archives, supports both stateless and stateful operations.
///
/// For more details on the Multifile format, see the [module documentation](self#format).
#[derive(Debug)]
#[allow(dead_code)]
pub struct Multifile {
    data: DataCursor,
    files: Vec<Subfile>,
    version: Version,
    timestamp: u32,
}

impl Multifile {
    /// Latest revision of the Multifile format. For more info, see [here](self#revisions).
    pub const CURRENT_VERSION: Version = Version { major: 1, minor: 1 };
    /// Unique identifier that tells us if we're reading a Multifile archive.
    pub const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    /// Helper function that reads the pre-header for a given file, if any, which allows for comment
    /// lines starting with '#'. Returns the position in the stream that the actual data starts.
    #[inline]
    fn parse_header_prefix(input: &[u8]) -> usize {
        let mut pos = 0;

        while pos < input.len() {
            //Check if a line starts with '#'
            if input[pos] == b'#' {
                //Look for the end of the line
                while pos < input.len() && input[pos] != b'\n' {
                    pos += 1;
                }
                //Skip any whitespace characters at the start of the next line
                while pos < input.len() && (input[pos] == b' ' || input[pos] == b'\r') {
                    pos += 1;
                }
            } else {
                //If it doesn't, we've found the end of the pre-header
                break;
            }
        }
        pos
    }

    /// Returns the metadata from a Multifile header.
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the magic number does not match a
    /// Multifile, or [`UnknownVersion`](Error::UnknownVersion) if the Multifile version is
    /// too new to be supported.
    #[inline]
    fn read_header<T: ReadExt>(data: &mut T) -> Result<Header> {
        //Read the magic and make sure we're actually parsing a Multifile
        let mut magic = [0u8; 6];
        data.read_length(&mut magic)?;
        ensure!(magic == Self::MAGIC, InvalidMagicSnafu);

        let version = Version { major: data.read_u16()?, minor: data.read_u16()? };
        ensure!(
            Self::CURRENT_VERSION.major == version.major && Self::CURRENT_VERSION.minor >= version.minor,
            UnknownVersionSnafu
        );

        let scale_factor = data.read_u32()?;

        let timestamp = match version.minor >= 1 {
            true => data.read_u32()?,
            false => 0,
        };

        Ok(Header { version, scale_factor, timestamp })
    }

    /// Returns the number of [`Subfile`]s currently stored in the Multifile.
    #[inline]
    pub fn count(&mut self) -> usize {
        self.files.len()
    }

    /// Opens a file on disk, loads its contents, and parses it into a new instance of
    /// Multifile. The returned instance can then be used for further operations.
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the magic number does not match a
    /// Multifile, [`UnknownVersion`](Error::UnknownVersion) if the Multifile version is too
    /// new to be supported, or [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(input: P, offset: u64) -> Result<Self> {
        let data = std::fs::read(input)?;
        Self::load(data, offset)
    }

    /// Loads the data from the given file and parses it into a new instance of Multifile. The
    /// returned instance can then be used for further operations.
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the magic number does not match a
    /// Multifile, [`UnknownVersion`](Error::UnknownVersion) if the Multifile version is too
    /// new to be supported, or [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I, offset: u64) -> Result<Self> {
        let mut data = DataCursor::new(input, Endian::Little);
        data.set_position(offset)?;
        data.set_position(Self::parse_header_prefix(&data) as u64)?;

        let header = Self::read_header(&mut data)?;
        let mut multifile = Self {
            data,
            files: Vec::new(),
            version: header.version,
            timestamp: header.timestamp,
        };

        // Loop through each Subfile, using next_index as a linked list
        let mut next_index = multifile.data.read_u32()? * header.scale_factor;
        while next_index != 0 {
            let mut subfile = Subfile::load(&mut multifile.data, header.version)?;
            subfile.offset *= header.scale_factor;
            if subfile.timestamp == 0 {
                subfile.timestamp = header.timestamp;
            }

            multifile.files.push(subfile);

            multifile.data.set_position(next_index.into())?;
            next_index = multifile.data.read_u32()? * header.scale_factor;
        }

        Ok(multifile)
    }

    /// Saves all [`Subfile`]s to disk. For use without having to [`open`](Self::open) or
    /// [`load`](Self::load), see [`extract_from`](Self::extract_from) and
    /// [`extract_from_path`](Self::extract_from_path).
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds, or an error if
    /// unable to create the necessary directories (see
    /// [`create_dir_all`](std::fs::create_dir_all)), or failing to create a file to write to (see
    /// [`write`](std::fs::write))
    #[inline]
    #[cfg(feature = "std")]
    pub fn extract_all<P: AsRef<Path>>(&mut self, output: P) -> Result<usize> {
        let mut saved_files = 0;
        for subfile in &mut self.files {
            if !subfile.flags.intersects(Flags::Signature | Flags::Compressed | Flags::Encrypted) {
                self.data.set_position(subfile.offset.into())?;
                subfile.write_file(&self.data.read_slice(subfile.length as usize)?, &output)?;
                saved_files += 1;
            }
        }
        Ok(saved_files)
    }

    /// Loads a Multifile from disk and extracts all [`Subfile`]s. For use with other functions,
    /// see [`extract`](Self::extract_all).
    ///
    /// # Errors
    /// Returns [`NotFound`](Error::NotFound) if the input file doesn't exist,
    /// [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds, or an error if unable to
    /// create the necessary directories (see [`create_dir_all`](std::fs::create_dir_all)), or
    /// failing to create a file to write to (see [`write`](std::fs::write)).
    #[cfg(feature = "std")]
    #[inline]
    pub fn extract_from_path<P: AsRef<Path>>(input: P, output: P, offset: u64) -> Result<()> {
        let data = std::fs::read(input)?;
        Self::extract_from(&data, output, offset)?;
        Ok(())
    }

    /// Extracts all [`Subfile`]s from the given Multifile. For use with other functions, see
    /// [`extract`](Self::extract_all).
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds, or an error if
    /// unable to create the necessary directories (see
    /// [`create_dir_all`](std::fs::create_dir_all)), or failing to create a file to write to (see
    /// [`write`](std::fs::write)).
    #[cfg(feature = "std")]
    #[inline]
    pub fn extract_from<P: AsRef<Path>>(input: &[u8], output: P, offset: u64) -> Result<()> {
        //Use a DataCursorRef internally because it makes reading structured data a lot easier
        let mut data = DataCursorRef::new(input, Endian::Little);
        data.set_position(offset)?;
        data.set_position(Self::parse_header_prefix(&data) as u64)?;

        let header = Self::read_header(&mut data)?;

        // Loop through each Subfile, using next_index as a linked list
        let mut next_index = data.read_u32()? * header.scale_factor;
        while next_index != 0 {
            let mut subfile = Subfile::load(&mut data, header.version)?;
            subfile.offset *= header.scale_factor;
            if subfile.timestamp == 0 {
                subfile.timestamp = header.timestamp;
            }

            data.set_position(subfile.offset.into())?;
            if !subfile.flags.contains(Flags::Signature) {
                subfile.write_file(&data.read_slice(subfile.length as usize)?, &output)?;
            } /* else if cfg!(signature) {
                  println!("{:?}", subfile);
                  data.set_position(subfile.offset as usize);
                  Self::check_signatures(data.get_slice(subfile.length as usize)?)?;
              }*/

            data.set_position(next_index.into())?;
            next_index = data.read_u32()? * header.scale_factor;
        }

        Ok(())
    }

    /// Parses file data containing Multifile signatures and certificate chains.
    ///
    /// Currently only useful to check that the signature data can be parsed correctly, does not
    /// verify the contents against the signature.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[cfg(feature = "signature")]
    #[inline]
    pub fn check_signatures(input: &[u8]) -> Result<()> {
        let mut file_data = DataCursor::new(input, Endian::Little);
        let signature_size = file_data.read_u32()?;
        file_data.set_position(4 + u64::from(signature_size))?;
        let cert_count = file_data.read_u32()?;
        let mut cert_blob = DataCursor::new(
            vec![0u8; (file_data.len()? - file_data.position()?) as usize],
            Endian::Little,
        );
        file_data.read_length(&mut cert_blob)?;

        for _ in 0..cert_count {
            let (_, remaining) = cert::read_certificate(&cert_blob.remaining_slice()?).unwrap();
            //println!("Certificate {n}:\n{certificate:?}");
            let length = cert_blob.len()?;
            cert_blob.set_position(length - remaining as u64)?;
        }
        Ok(())
    }
}

#[cfg(feature = "identify")]
impl FileIdentifier for Multifile {
    fn identify(data: &[u8]) -> Option<FileInfo> {
        let multifile = Self::load(data, 0).ok()?;
        let (num_compressed, num_encrypted) = multifile.files.iter().fold((0, 0), |(comp, enc), subfile| {
            let is_compressed = subfile.flags.contains(Flags::Compressed) as usize;
            let is_encrypted = subfile.flags.contains(Flags::Encrypted) as usize;
            (comp + is_compressed, enc + is_encrypted)
        });

        //u32 will always be inside i64::MAX, so we can unwrap. We'll worry about it in 2106.
        let timestamp = time::format_timestamp(multifile.timestamp.into()).unwrap();

        let mut info = format!(
            "Panda3D Multifile archive v{}, modified {}, file count: {}",
            multifile.version,
            timestamp,
            multifile.files.len()
        );

        //Manually build additional details
        let details = format!(
            "{}{}{}",
            if num_compressed > 0 {
                format!("{num_compressed} compressed")
            } else {
                String::new()
            },
            if num_compressed > 0 && num_encrypted > 0 {
                ", "
            } else {
                ""
            },
            if num_encrypted > 0 {
                format!("{num_encrypted} encrypted")
            } else {
                String::new()
            }
        );

        if details.is_empty() {
            info.push('.');
        } else {
            info.push_str(&format!(" ({details})."));
        }

        Some(FileInfo::new(info, None))
    }
}
