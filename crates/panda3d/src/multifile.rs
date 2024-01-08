//! Adds support for the Multifile archive format used by the Panda3D engine.
//!
//! This module is designed to support both "standalone" or one-shot operations that don't require
//! you to keep track of an object, and holding a Multifile in-memory for more complex operations.
//!
//! # Revisions
//! * **Version 1.0**: Initial Multifile Support
//! * **Version 1.1**: Added support for timestamps both for the Multifile as a whole, and
//! individual Subfiles. Subfiles with a timestamp of zero will use the Multifile timestamp.
//!
//! # Format
//! The Multifile format is designed as a header, and then a number of [`Subfile`]s connected via a
//! linked list structure.
//!
//! It has primitive support for larger-than-32bit filesizes, using a "scale factor" that multiplies
//! all offsets by a specific amount, which introduces extra padding to meet alignment requirements.
//!
//! It supports comment lines before the Multifile data using a '#' at the start which allows a
//! Multifile to be ran directly from the command line on Unix systems.
//!
//! ## Multifile Header
//! The header is as follows, in little-endian format:
//!
//! | Offset | Field | Type | Notes |
//! |--------|-------|------|-------|
//! | 0x0     | Magic number   | u8\[6] | Unique identifier ("pmf\\0\\n\\r") to let us know we're reading a Multifile. |
//! | 0x4     | Major version  | u16    | Major version number for the Multifile revision (currently 1). |
//! | 0x8     | Minor version  | u16    | Minor version number for the Multifile revision (currently 1). |
//! | 0xC     | Scale factor   | u32    | Allows for scaling all file offsets to support larger-than-4GB files. |
//! | \[0x10] | Unix timestamp | u32    | The time the Multifile was last modified ***(revision 1.1+)***. |
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
//! | 0x14    | Subfile path    | char\[len] | Path to the Subfile, for use in a Virtual FileSystem. Obfuscated, convert as (255 - x). |
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
//! # Usage
//! This module can be used either with borrowed data or as an in-memory archive.
//!
//! ## Stateful Functions
//! A [`Multifile`] can be created through [`open`](Multifile::open), which will read a file from
//! disk, and [`load`](Multifile::load), which will read the provided file.
//!
//! Once created, the following functions can be used to manipulate the archive:
//!
//! * [`extract`](Multifile::extract): Save all contained [`Subfile`]s to a given folder
//!
//! ## Stateless Functions
//! These functions can be used without having to first create a [`Multifile`], used for the
//! following one-shot operations:
//!
//! * [`extract_from_path`](Multifile::extract_from_path): Reads a [`Multifile`] from disk, and saves all [`Subfile`]s to a given folder
//! * [`extract_from`](Multifile::extract_from): Reads the provided [`Multifile`], and saves all [`Subfile`]s to a given folder

use core::fmt;
#[cfg(feature = "std")]
use std::io::prelude::*;
#[cfg(feature = "std")]
use std::path::Path;

use der::Decode;
use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::subfile::*;

#[cfg(not(feature = "std"))]
use crate::no_std::*;

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
    #[snafu(display(
        "Unknown Multifile Version! Expected >= v{}.",
        Multifile::CURRENT_VERSION
    ))]
    UnknownVersion,
    #[snafu(display("Tried to do an operation without any data to work with."))]
    NoData,
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
                panic!(
                    "Unexpected std::io::error: {}! Something has gone horribly wrong",
                    kind
                )
            }
        }
    }
}

impl From<data::Error> for Error {
    #[inline]
    fn from(error: data::Error) -> Self {
        match error {
            data::Error::EndOfFile => Self::EndOfFile,
            _ => panic!("Unexpected data::error! Something has gone horribly wrong"),
        }
    }
}

/// This struct is mainly for readability in place of an unnamed tuple
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Version {
    pub major: i16,
    pub minor: i16,
}

impl fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

struct Header {
    version: Version,
    scale_factor: u32,
    timestamp: u32,
}

//The current least terrible way to implement state in this system is to just store the entire
//Multifile data, and have each Subfile keep an offset+length. In the future, once safe transmute is
//a thing, I can "take" each header and what's left will be all the relevant file data.

#[allow(dead_code)]
pub struct Multifile {
    data: DataCursor,
    files: Vec<Subfile>,
    version: Version,
    timestamp: u32,
}

impl Multifile {
    pub const CURRENT_VERSION: Version = Version { major: 1, minor: 1 };
    pub const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    /// Helper function that reads the pre-header for a given file, if any, which allows for comment
    /// lines starting with '#'.
    #[inline]
    fn parse_header_prefix(input: &[u8]) -> usize {
        let mut pos = 0;

        while pos < input.len() {
            //Check if a line starts with '#'
            if input[pos] == b'#' {
                //If it doesn't, look for the next newline
                while pos < input.len() && input[pos] != b'\n' {
                    pos += 1;
                }
                //Skip any whitespace characters at the start of the line
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

    #[inline]
    fn read_header<T: EndianRead + Read>(data: &mut T) -> Result<Header> {
        //Read the magic and make sure we're actually parsing a Multifile
        let mut magic = [0u8; 6];
        data.read_exact(&mut magic)?;
        ensure!(magic == Self::MAGIC, InvalidMagicSnafu);

        let version = Version {
            major: data.read_i16()?,
            minor: data.read_i16()?,
        };
        ensure!(
            Self::CURRENT_VERSION.major == version.major
                && Self::CURRENT_VERSION.minor >= version.minor,
            UnknownVersionSnafu
        );

        let scale_factor = data.read_u32()?;

        let timestamp = match version.minor >= 1 {
            true => data.read_u32()?,
            false => 0,
        };
        Ok(Header {
            version,
            scale_factor,
            timestamp,
        })
    }

    #[inline]
    pub fn count(&mut self) -> usize {
        self.files.len()
    }

    #[cfg(feature = "std")]
    #[inline]
    #[must_use]
    pub fn open<P: AsRef<Path>>(input: P, offset: usize) -> Result<Self> {
        let mut data = DataCursor::new(std::fs::read(input)?, Endian::Little);
        data.set_position(offset);
        data.set_position(Self::parse_header_prefix(&data));

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

            multifile.data.set_position(next_index as usize);
            next_index = multifile.data.read_u32()? * header.scale_factor;
        }

        Ok(multifile)
    }

    #[inline]
    #[must_use]
    pub fn load<I: Into<Box<[u8]>>>(input: I, offset: usize) -> Result<Self> {
        let mut data = DataCursor::new(input, Endian::Little);
        data.set_position(offset);
        data.set_position(Self::parse_header_prefix(&data));

        let multifile = Self {
            data,
            files: Vec::new(),
            version: Self::CURRENT_VERSION,
            timestamp: 0,
        };

        Ok(multifile)
    }

    #[inline]
    pub fn extract<P: AsRef<Path>>(&mut self, output: P) -> Result<usize> {
        let mut saved_files = 0;
        for subfile in &mut self.files {
            if !subfile.flags.intersects(Flags::Signature | Flags::Compressed | Flags::Encrypted) {
                self.data.set_position(subfile.offset as usize);
                subfile.write_file(self.data.get_slice(subfile.length as usize)?, &output)?;
                saved_files += 1;
            }
        }
        Ok(saved_files)
    }

    /// Loads a [`Multifile`] and extracts all [`Subfiles`](self::Subfile). For use with other
    /// functions, see [`extract`](Self::extract).
    #[cfg(feature = "std")]
    #[inline]
    pub fn extract_from_path<P: AsRef<Path>>(input: P, output: P, offset: usize) -> Result<()> {
        let data = std::fs::read(input)?;
        Self::extract_from(&data, output, offset)?;
        Ok(())
    }

    /// Extracts all Subfiles from the given Multifile. For use with other functions, see
    /// [`extract`](Self::extract).
    #[cfg(feature = "std")]
    #[inline]
    pub fn extract_from<P: AsRef<Path>>(input: &[u8], output: P, offset: usize) -> Result<()> {
        //Use a DataCursorRef internally because it makes reading structured data a lot easier
        let mut data = DataCursorRef::new(input, Endian::Little);
        data.set_position(offset);
        data.set_position(Self::parse_header_prefix(&data));

        let header = Self::read_header(&mut data)?;

        // Loop through each Subfile, using next_index as a linked list
        let mut next_index = data.read_u32()? * header.scale_factor;
        while next_index != 0 {
            let mut subfile = Subfile::load(&mut data, header.version)?;
            subfile.offset *= header.scale_factor;
            if subfile.timestamp == 0 {
                subfile.timestamp = header.timestamp;
            }

            data.set_position(subfile.offset as usize);
            if !subfile.flags.contains(Flags::Signature) {
                subfile.write_file(data.get_slice(subfile.length as usize)?, &output)?;
            } else {
                println!("{:?}", subfile);
                data.set_position(subfile.offset as usize);
                Self::check_signatures(data.get_slice(subfile.length as usize)?)?;
            }

            data.set_position(next_index as usize);
            next_index = data.read_u32()? * header.scale_factor;
        }

        Ok(())
    }

    #[inline]
    pub fn check_signatures(input: &[u8]) -> Result<()> {
        let mut file_data = DataCursor::new(input, Endian::Little);
        let signature_size = file_data.read_u32()?;
        file_data.set_position(4 + signature_size as usize);
        let certificate_count = file_data.read_u32()?;
        let mut certificate_blob = DataCursor::new(
            vec![0u8; file_data.len() - file_data.position()],
            Endian::Little,
        );
        file_data.read_exact(&mut certificate_blob)?;

        for n in 0..certificate_count {
            let certificate = orthrus_core::certificate::Certificate::from_der(
                certificate_blob.remaining_slice(),
            )
            .unwrap();
            println!("Certificate {n}:\n{certificate:?}");
            let remaining_length: usize = certificate.remaining_len.try_into().unwrap();
            certificate_blob.set_position(certificate_blob.len() - remaining_length);
        }
        Ok(())
    }

    //Multifile::check_signatures parses and verifies the signature and certificates
}

/*use core::fmt;
use std::io::prelude::*;
use std::path::Path;

//use orthrus_core::prelude::*;
//use orthrus_core::{data, time};
use orthrus_core::prelude::*;
use snafu::prelude::*;

/// This struct is mainly for readability in place of an unnamed tuple
#[derive(PartialEq, Eq)]
pub struct Version {
    major: i16,
    minor: i16,
}

impl fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum MultifileError {
    #[snafu(display("Unable to find file/folder!"))]
    NotFound,
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    #[snafu(display("Invalid End Size!"))]
    InvalidSize,
    #[snafu(display("Unable to convert timestamp!"))]
    InvalidTimestamp,
    #[snafu(display("Invalid Magic! Expected {expected:?}"))]
    InvalidMagic { expected: [u8; 6] },
    #[snafu(display("Unknown Multifile Version! Expected v{major}.{minor}"))]
    UnknownVersion { major: i16, minor: i16 },
}

#[cfg(feature = "std")]
impl From<std::io::Error> for MultifileError {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::UnexpectedEof => Self::EndOfFile,
            _ => panic!("Unexpected std::io::error! Something has gone horribly wrong"),
        }
    }
}

impl From<data::Error> for MultifileError {
    #[inline]
    fn from(error: data::Error) -> Self {
        match error {
            data::Error::EndOfFile => Self::EndOfFile,
            data::Error::InvalidSize => Self::InvalidSize,
            _ => panic!("Unexpected data::error! Something has gone horribly wrong"),
        }
    }
}

impl From<time::Error> for MultifileError {
    #[inline]
    fn from(error: time::Error) -> Self {
        match error {
            time::Error::ComponentRange(_) => Self::EndOfFile,
            time::Error::IndeterminateOffset(_) => Self::InvalidTimestamp,
            _ => panic!("Unexpected time::error! Something has gone horribly wrong"),
        }
    }
}

pub struct Multifile {}

/// This is a test, thanks
impl Multifile {
    pub const CURRENT_VERSION: Version = Version { major: 1, minor: 1 };
    pub const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    /// Parses a `Panda3D` Multifile pre-header, which allows for comment lines starting with '#'.
    ///
    /// Returns either a [String] containing the header comment data, or an [`IOError`](Error::Io)
    /// if it reaches EOF before finding the Multifile magic ("pmf\0\n\r").

    ///This function tries to read a pre-header, if any, which allows for comment lines starting
    /// with '#'.
    /*fn parse_header_prefix(input: &[u8]) -> Result<&[u8], MultifileError> {
        let mut header_prefix = String::new();
        let mut line = String::new();

        loop {
            if input.read_u8()? == b'#' {
                let len = input.read_line(&mut line)?;
                if len == 0 {
                    return Err(MultifileError::EndOfFile);
                }

                header_prefix.push('#');
                header_prefix.push_str(&line);

                line.clear();
            } else {
                input.set_position(input.position() - 1);
                return Ok(header_prefix);
            }
        }
    }*/

    #[cfg(feature = "std")]
    #[inline]
    pub fn extract_from_path<P>(input: P, output: P, offset: usize) -> Result<(), MultifileError>
    where
        P: AsRef<Path> + fmt::Display,
    {
        log::info!("Reading Multifile from {input}");
        let data = std::fs::read(input)?;
        Self::extract_from(&data, output, offset)
    }

    /// This function is designed to run as a single pass. For more general use, see
    /// [`extract`](Multifile::extract).
    #[cfg(feature = "std")]
    #[inline]
    pub fn extract_from<P>(input: &[u8], output: P, offset: usize) -> Result<(), MultifileError>
    where
        P: AsRef<Path> + fmt::Display,
    {
        log::info!("Extracting Multifile to {output}, using offset {offset}");

        let mut data = DataCursor::new(input, Endian::Little);

        let mut magic = [0u8; 6];
        data.read_exact(&mut magic).map_err(|_| MultifileError::EndOfFile)?;

        if magic != Self::MAGIC {
            let error = MultifileError::InvalidMagic {
                expected: Self::MAGIC,
            };
            log::error!("{error}");
            return Err(error);
        }

        // Latest version is v1.1
        let version = Version {
            major: data.read_i16()?,
            minor: data.read_i16()?,
        };

        if version != Self::CURRENT_VERSION {
            let error = MultifileError::UnknownVersion {
                major: Self::CURRENT_VERSION.major,
                minor: Self::CURRENT_VERSION.minor,
            };
            log::error!("{error}");
            return Err(error);
        }

        log::info!("Multifile v{version}");

        // Only print scale factor if it's bigger than 1 since that's the norm
        let scale_factor = data.read_u32()?;
        if scale_factor > 1 {
            log::info!("Scale Factor: {scale_factor}");
        }

        // Timestamp added in v1.1
        if version.minor >= 1 {
            let timestamp = data.read_u32()?;
            log::info!(
                "Last Modified: {} ({timestamp})",
                time::format_timestamp(timestamp.into())?
            );
        }

        Ok(())
    }

    /// This function
    #[inline]
    pub fn extract<P: AsRef<Path> + fmt::Display>(
        &mut self,
        _output: P,
    ) -> Result<(), MultifileError> {
        Ok(())
    }
}*/

/*
use core::fmt;
use core::str::from_utf8;
use std::io::prelude::*;
use std::path::Path;

use bitflags::bitflags;
use compact_str::CompactString;
use orthrus_core::certificate::{print_x509_info, Certificate};
use orthrus_core::prelude::*;
use orthrus_core::time;
//use orthrus_core::vfs::VirtualFolder;

/// This struct is mainly for readability in place of an unnamed tuple
#[derive(PartialEq)]
struct Version {
    major: i16,
    minor: i16,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A Multifile is a container format used by `Panda3D` to distribute assets, similar to a .zip.
///
/// # Multifile format
/// The Multifile specification (currently v1.1, little-endian) is as follows:
///
/// | Offset | Field | Type | Length | Notes |
/// |---|---|---|---|---|
/// | 0 | Magic number | u8\[6\] | 6 | The unique identifier to let us know we're reading a Multifile (always "pmf\0\n\r"). |
/// | 6 | Major version | i16 | 2 | The major version of the Multifile format (currently 1). |
/// | 8 | Minor version | i16 | 2 | The minor version of the Multifile format (currently 1). |
/// | 10 | Scale factor | u32 | 4 | To allow for Multifiles larger than a 32-bit integer (4,294,967,295 bytes), this will "scale" all offsets. |
/// | 14 | Unix timestamp | u32 | 4 | The date the Multifile was last modified. |
///
/// ## Subfile Format
/// Following the Multifile header is a number of Subfiles. In order to parse the Multifile body,
/// you read an index offset pointing to the next Subfile header and follow it until you reach a 0
/// which indicates the end of the Multifile.
///
/// The Subfile format (v1.1) is as follows:
///
/// | Offset | Field | Type | Length | Notes |
/// |---|---|---|---|---|
/// | 0 | Index offset | u32 | 4 | Offset to the next header relative to file start. Multiply by the scale factor to get the correct offset. If it's 0, there are no more Subfiles. |
/// | 4 | Data offset | u32 | 4 | Offset to the actual Subfile data relative to file start. Multiply by the scale factor to get the correct offset. |
/// | 8 | Data length | u32 | 4 | The length of the Subfile data. |
/// | 12 | Subfile flags | u16 | 2 | These are special attributes attached to a Subfile, such as compression or encryption. See [Subfile flags](#subfile-flags) for a complete list. |
/// | 14 | Original length | u32 | 4 | Only in the Subfile header if the Subfile is compressed or encrypted! This is the length of the data after being decompressed/decrypted. |
/// | 18 | Unix timestamp | u32 | 4 | The date the Subfile was last modified. If it's 0, use the Multifile timestamp. |
/// | 20 | Name length | u16 | 2 | Length of the Subfile's name. Read this many more bytes. |
/// | 22 | Subfile path | char\[len\] | len | This is the path to the actual Subfile, used as part of a Virtual Filesystem. Obfuscated, convert each character as (255 - x). |
///
/// ## Subfile Flags
/// | Flag | Value | Notes |
/// |---|---|---|
/// | `Deleted` | 1 << 0 (1) | The Subfile has been deleted from the Multifile, so it should be ignored. |
/// | `IndexInvalid` | 1 << 1 (2) | The Subfile has a corrupt index entry. |
/// | `DataInvalid` | 1 << 2 (4) | The Subfile has invalid data. |
/// | `Compressed` | 1 << 3 (8) | The Subfile is compressed. |
/// | `Encrypted` | 1 << 4 (16) | The Subfile is encrypted. |
/// | `Signature` | 1 << 5 (32) | The Subfile is the certificate used to sign the Multifile. See [Certificate format](#certificate-format) for more details. |
/// | `Text` | 1 << 6 (64) | The Subfile contains text instead of binary data. |
///
/// ## Certificate Format
/// The Multifile certificate is a binary blob that can contain multiple certificates, along with
/// the actual signature of the Multifile. It starts with the length of the signature (u32), and
/// then the actual signature. After that is the number of certificates (u32), and then a blob
/// containing all certificates. The way it is meant to be read is to repeatedly call `d2i_X509` or
/// an equivalent function that allows you to get the remaining bytes after parsing a certificate.
pub struct Multifile {
    //root: VirtualFolder,
    version: Version,
    scale_factor: u32,
    timestamp: u32,
}

impl Multifile {
    const CURRENT_VERSION: Version = Version { major: 1, minor: 1 };
    const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    /// Parses a `Panda3D` Multifile pre-header, which allows for comment lines starting with '#'.
    ///
    /// Returns either a [String] containing the header comment data, or an [`IOError`](Error::Io)
    /// if it reaches EOF before finding the Multifile magic ("pmf\0\n\r").
    fn parse_header_prefix(input: &mut DataCursor) -> Result<String> {
        let mut header_prefix = String::new();
        let mut line = String::new();

        loop {
            if input.read_u8()? == b'#' {
                let len = input.read_line(&mut line)?;
                if len == 0 {
                    return Err(Error::EndOfFile);
                }

                header_prefix.push('#');
                header_prefix.push_str(&line);

                line.clear();
            } else {
                input.set_position(input.position() - 1);
                return Ok(header_prefix);
            }
        }
    }

    pub fn from_path<P: AsRef<Path>>(path: P, offset: usize) -> Result<Self> {
        // Acquire file data
        let mut data = DataCursor::from_path(path, Endian::Little)?;
        data.set_position(offset);

        // Check if there's any pre-header
        let header_text = Self::parse_header_prefix(&mut data)?;
        if !header_text.is_empty() {
            log::info!("Multifile Pre-Header:\n{}\n", header_text);
        }

        // Parse Multifile data
        let mut multifile = Self::default();
        multifile.read_index(&mut data)?;

        // Print the actual filesystem out to debug
        //log::debug!("{}", multifile.root);

        Ok(multifile)
    }

    pub fn read_index(&mut self, input: &mut DataCursor) -> Result<()> {
        log::trace!("Reading Index, Offset {:#X}", input.position());

        // Parse the Multifile header
        let mut magic = [0u8; 6];
        input.read_exact(&mut magic)?;

        if magic != Self::MAGIC {
            let error = Error::InvalidMagic {
                expected: format!("{:?}", from_utf8(&Self::MAGIC)?).into(),
            };
            log::error!("{}", error);
            return Err(error);
        }

        // Latest version is v1.1
        self.version = Version {
            major: input.read_i16()?,
            minor: input.read_i16()?,
        };

        if self.version != Self::CURRENT_VERSION {
            let error = Error::UnknownVersion {
                expected: Self::CURRENT_VERSION.to_string().into(),
            };
            log::error!("{}", error);
            return Err(error);
        }

        log::info!("Multifile v{}", self.version);

        // Only print scale factor if it's bigger than 1 since that's the norm
        self.scale_factor = input.read_u32()?;
        if self.scale_factor > 1 {
            log::info!("Scale Factor: {}", self.scale_factor);
        }

        // Timestamp added in v1.1
        if self.version.minor >= 1 {
            self.timestamp = input.read_u32()?;
            log::info!(
                "Last Modified: {} ({})",
                time::format_timestamp(self.timestamp.into())?,
                self.timestamp
            );
        }

        log::trace!("Starting Subfile Parse!");
        // Loop through each Subfile, using next_index as a linked list
        let mut next_index = input.read_u32()? * self.scale_factor;
        while next_index != 0 {
            log::trace!(
                "Reading Subfile Header at Offset {:#X}, Next Index at {:#X}",
                input.position(),
                next_index
            );
            let (mut subfile, _filename) = Subfile::from_data(input, self)?;

            if subfile.flags.contains(SubfileFlags::Signature) {
                subfile.parse_signature()?;
            } else {
                //self.root.create_file(filename.split('/').peekable(), subfile);
            }

            input.set_position(next_index as usize);
            next_index = input.read_u32()? * self.scale_factor;
        }
        Ok(())
    }

    //pub fn extract<P: AsRef<Path>>(&self, output: P) {}
}

impl Default for Multifile {
    fn default() -> Self {
        Self {
            //root: VirtualFolder::default(),
            version: Self::CURRENT_VERSION,
            scale_factor: 1,
            timestamp: 0,
        }
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Default)]
    struct SubfileFlags: u16 {
        const Deleted = 1 << 0;
        const IndexInvalid = 1 << 1;
        const DataInvalid = 1 << 2;
        const Compressed = 1 << 3;
        const Encrypted = 1 << 4;
        const Signature = 1 << 5;
        const Text = 1 << 6;
    }
}

impl core::fmt::Display for SubfileFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        if self.is_empty() {
            write!(f, "")
        } else {
            write!(f, "{}", &self.0)
        }
    }
}

#[derive(Default)]
struct Subfile {
    data: DataCursor,
    timestamp: u32,
    flags: SubfileFlags,
}

impl Subfile {
    pub fn from_data(
        data: &mut DataCursor,
        multifile: &Multifile,
    ) -> Result<(Self, CompactString)> {
        let mut subfile = Self::default();
        let filename = subfile.read_index(data, multifile)?;
        Ok((subfile, filename))
    }

    /// This function takes a [`DataCursor`] to read data from and the parent [`Multifile`] for
    /// global settings, and populates the `Subfile` data.
    pub fn read_index(
        &mut self,
        input: &mut DataCursor,
        multifile: &Multifile,
    ) -> Result<CompactString> {
        let offset = input.read_u32()? * multifile.scale_factor;
        let data_length = input.read_u32()?;
        self.flags = SubfileFlags::from_bits_truncate(input.read_u16()?);

        let length;
        if self.flags.intersects(SubfileFlags::Compressed | SubfileFlags::Encrypted) {
            length = input.read_u32()?;
            log::debug!("Subfile is compressed or encrypted! Original length: {length}");
        } else {
            length = data_length;
        }

        self.timestamp = input.read_u32()?;
        if self.timestamp == 0 {
            self.timestamp = multifile.timestamp;
        }

        let name_length = input.read_u16()?;
        let mut filename = CompactString::default();
        filename.reserve(name_length.into());

        for _ in 0..name_length {
            filename.push((255 - input.read_u8()?) as char);
        }

        log::info!(
            "Offset: {offset:>#8X} | Length: {data_length:>#8X}{}{}",
            if filename.is_empty() {
                String::new()
            } else {
                format!(" | Filename: {filename}")
            },
            if self.flags.is_empty() {
                String::new()
            } else {
                format!(" | Subfile flags: {}", self.flags)
            }
        );

        self.read_data(input, offset as usize, length as usize)?;

        Ok(filename)
    }

    pub fn read_data(
        &mut self,
        input: &mut DataCursor,
        offset: usize,
        length: usize,
    ) -> Result<()> {
        log::trace!("Reading data! Offset {:#X}, Length {:#X}", offset, length);
        input.set_position(offset);

        self.data = DataCursor::new(vec![0u8; length], Endian::Little);
        input.read_exact(self.data.as_mut())?;
        Ok(())
    }

    pub fn parse_signature(&mut self) -> Result<Vec<Certificate>> {
        // Signature begins with u32 length, u8[] data
        let signature_length = self.data.read_u32()?;
        self.data.set_position(self.data.position() + signature_length as usize);

        // Next is number of certificates, followed by raw data blob
        let num_certificates = self.data.read_u32()?;

        let data_blob = self.data.remaining_slice().to_vec();
        let mut certificates = Vec::new();
        let mut offset = 0;

        for certificate_number in 1..=num_certificates {
            let certificate = Certificate::from_der(&data_blob[offset..])?;

            log::debug!(
                "Certificate {}\n{}",
                certificate_number,
                print_x509_info(certificate.cert())?
            );

            offset += certificate.len();
            certificates.push(certificate);
        }

        self.data.set_position(0);
        Ok(certificates)
    }
}
*/
