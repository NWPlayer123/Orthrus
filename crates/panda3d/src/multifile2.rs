#[cfg(feature = "std")]
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[cfg(not(feature = "std"))]
use crate::no_std::*;

use bitflags::bitflags;
use orthrus_core::prelude::*;
use snafu::prelude::*;

/// Error conditions when working with Multifile archives.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "std")]
    #[snafu(display("Filesystem Error {}", source))]
    FileError { source: std::io::Error },

    /// Thrown if trying to read the file out of its current bounds.
    #[snafu(display("Reached the end of the current stream!"))]
    EndOfFile,

    /// Thrown if the header contains a magic number other than "pmf\0\n\r".
    #[snafu(display("Invalid Magic! Expected {:?}.", Multifile::MAGIC))]
    InvalidMagic,

    /// Thrown if the header version is too new to be supported.
    #[snafu(display("Unknown Multifile Version! Expected >= v{}.", Multifile::CURRENT_VERSION))]
    UnknownVersion,
}

impl From<DataError> for Error {
    #[inline]
    fn from(error: DataError) -> Self {
        match error {
            #[cfg(feature = "std")]
            DataError::Io { source } => Self::FileError { source },
            DataError::EndOfFile => Self::EndOfFile,
            _ => todo!(),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        Error::FileError { source: error }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Version {
    major: u16,
    minor: u16,
}

impl core::fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug)]
pub struct Header {
    version: Version,
    scale_factor: u32,
    timestamp: u32,
}

/// This is used internally to reduce allocations due to the full Multifile allocating additional space for
/// each file's data.
#[derive(Debug)]
struct Metadata {
    header: Header,
    files: Vec<SubfileHeader>,
}

#[derive(Debug)]
pub struct Multifile {
    header: Header,
    files: BTreeMap<String, Subfile>,
}

impl Multifile {
    /// Latest revision of the Multifile format. For more info, see [here](self#revisions).
    pub const CURRENT_VERSION: Version = Version { major: 1, minor: 1 };
    /// Unique identifier that tells us if we're reading a Multifile archive.
    pub const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    /// Helper function that searches for the start of the actual Multifile, skipping any header prefix, which
    /// allow for comment lines starting with '#'. Returns the size of the header prefix.
    #[inline]
    fn parse_header_prefix<T: ReadExt + SeekExt>(data: &mut T) -> Result<u64, self::Error> {
        let mut pos = 0;

        loop {
            // This checks the initial byte of the line, if at any point we error, just return 0;
            match data.read_u8()? {
                b'#' => {
                    pos += 1;
                    // If we are in a comment line, search for a '\n'
                    loop {
                        let byte = data.read_u8()?;
                        pos += 1;
                        if byte == b'\n' {
                            break;
                        }
                    }
                    // Once we find the end of a line, skip any whitespace at the start of the next line
                    loop {
                        let byte = data.read_u8()?;
                        if !matches!(byte, b' ' | b'\r') {
                            let position = data.position()?;
                            data.set_position(position - 1)?;
                            break;
                        }
                        pos += 1;
                    }
                }
                _ => break,
            }
        }
        Ok(pos)
    }

    /// Returns the data from a `Multifile` header.
    #[inline]
    fn read_header<T: ReadExt>(data: &mut T) -> Result<Header, self::Error> {
        let magic = data.read_slice(6)?;
        ensure!(*magic == Self::MAGIC, InvalidMagicSnafu);

        let version = Version { major: data.read_u16()?, minor: data.read_u16()? };
        ensure!(
            Self::CURRENT_VERSION.major == version.major && Self::CURRENT_VERSION.minor >= version.minor,
            UnknownVersionSnafu
        );

        let scale_factor = data.read_u32()?;

        let timestamp = match version.major >= 1 {
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

    /// Opens a file on disk, loads its contents, and parses it into a new `Multifile` instance. The instance
    /// can then be used for further operations.
    #[inline]
    #[cfg(feature = "std")]
    pub fn open<P: AsRef<Path>>(path: P, offset: u64) -> Result<Self, self::Error> {
        let data = BufReader::new(File::open(path)?);
        Multifile::load(data, offset)
    }

    /// Loads the data from a given input and parses it into a new `Multifile` instance. The instance can then
    /// be used for further operations.
    #[inline]
    pub fn load<T: IntoDataStream>(input: T, offset: u64) -> Result<Self, self::Error> {
        let mut data = input.into_stream(Endian::Little);
        data.set_position(offset)?;
        let metadata = Self::load_metadata(&mut data)?;

        // Now, let's actually build our sorted list of files (ideally, this will already be sorted inside
        // the Multifile)
        let mut files = BTreeMap::new();
        for mut header in metadata.files {
            // First, let's verify that our optional parameters are valid
            if header.timestamp == 0 {
                header.timestamp = metadata.header.timestamp;
            }
            if header.original_length == 0 {
                header.original_length = header.length;
            }

            let subfile = Subfile::load(&mut data, &header)?;
            files.insert(header.filename, subfile);
        }

        Ok(Multifile { header: metadata.header, files })
    }

    /// Loads the entire `Multifile` metadata and returns it as an instance. Used for sharing a ReadExt +
    /// SeekExt stream across multiple operations.
    ///
    /// This assumes that the input data is already at the start of a Multifile, i.e. we've already skipped
    /// any potential header prefix or offset.
    fn load_metadata<T: ReadExt + SeekExt>(data: &mut T) -> Result<Metadata, self::Error> {
        let header = Multifile::read_header(data)?;

        // This is designed to work with an "optimized" Multifile. This means that all Subfile metadata is at
        // the beginning of the file, with the actual file data following, so that seeking is minimized. Here,
        // we accumulate all metadata, and then parse it into a BTreeMap to optimize cache efficiency.
        let mut files = Vec::new();

        let mut next_index = data.read_u32()? * header.scale_factor;
        while next_index != 0 {
            let subfile = SubfileHeader::load(data, header.version)?;
            files.push(subfile);

            data.set_position(next_index.into())?;
            next_index = data.read_u32()? * header.scale_factor;
        }

        Ok(Metadata { header, files })
    }

    /// Extracts all non-special Subfiles to the specified output directory.
    #[inline]
    #[cfg(feature = "std")]
    pub fn extract_all<P: AsRef<Path>>(&mut self, output: P) -> Result<usize, self::Error> {
        let output = PathBuf::from(output.as_ref());
        let mut saved_files = 0;
        for subfile in &self.files {
            if !subfile
                .1
                .attributes
                .intersects(Attributes::Signature | Attributes::Compressed | Attributes::Encrypted)
            {
                let path = output.join(subfile.0);

                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir)?;
                }

                let mut file = File::create(path)?;
                file.write_all(&subfile.1.data)?;
                if subfile.1.timestamp != 0 {
                    let timestamp = Duration::from_secs(subfile.1.timestamp.into());
                    if let Some(modified) = SystemTime::UNIX_EPOCH.checked_add(timestamp) {
                        file.set_modified(modified)?;
                    }
                }

                saved_files += 1;
            }
        }
        Ok(saved_files)
    }

    #[inline]
    #[cfg(feature = "std")]
    pub fn extract_from_file<P: AsRef<Path>>(input: P, output: P) -> Result<usize, self::Error> {
        let input = BufReader::new(File::open(input.as_ref())?);
        let mut data = DataStream::new(input, Endian::Little);
        let output = PathBuf::from(output.as_ref());

        // Load all metadata (hopefully at the beginning of the file so our BufReader isn't getting thrashed)
        let metadata = Self::load_metadata(&mut data)?;

        // Now, let's actually extract to the filesystem
        let mut saved_files = 0;
        for mut header in metadata.files {
            // First, let's verify that our optional parameters are valid
            // TODO: if we're on version 1.0, grab the current timestamp as a placeholder?
            // TODO: We also should probably set the timestamp in the filesystem
            if metadata.header.version.minor == 0 {}

            if header.timestamp == 0 {
                header.timestamp = metadata.header.timestamp;
            }
            if header.original_length == 0 {
                header.original_length = header.length;
            }

            if !header
                .attributes
                .intersects(Attributes::Signature | Attributes::Compressed | Attributes::Encrypted)
            {
                let path = output.join(header.filename);

                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir)?;
                }

                data.set_position(header.offset.into())?;

                let mut file = File::create(path)?;
                file.write_all(&data.read_slice(header.length as usize)?)?;
                if header.timestamp != 0 {
                    let timestamp = Duration::from_secs(header.timestamp.into());
                    if let Some(modified) = SystemTime::UNIX_EPOCH.checked_add(timestamp) {
                        file.set_modified(modified)?;
                    }
                }

                saved_files += 1;
            }
        }

        Ok(saved_files)
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    struct Attributes: u16 {
        const Deleted = 1 << 0;
        const IndexInvalid = 1 << 1;
        const DataInvalid = 1 << 2;
        const Compressed = 1 << 3;
        const Encrypted = 1 << 4;
        const Signature = 1 << 5;
        const Text = 1 << 6;
    }
}

#[derive(Debug)]
struct SubfileHeader {
    offset: u32,
    length: u32,
    attributes: Attributes,
    original_length: u32,
    timestamp: u32,
    filename: String,
}

impl SubfileHeader {
    #[inline]
    fn load<T: ReadExt>(data: &mut T, version: Version) -> Result<Self, self::Error> {
        let offset = data.read_u32()?;
        let length = data.read_u32()?;
        let attributes = Attributes::from_bits_truncate(data.read_u16()?);

        let original_length = match attributes.intersects(Attributes::Compressed | Attributes::Encrypted) {
            true => data.read_u32()?,
            false => length,
        };

        let timestamp = match version.minor >= 1 {
            true => data.read_u32()?,
            false => 0,
        };

        let name_length = data.read_u16()?;
        let mut filename = String::with_capacity(name_length.into());
        for c in &*data.read_slice(name_length.into())? {
            filename.push((255 - *c).into());
        }

        Ok(SubfileHeader { offset, length, attributes, original_length, timestamp, filename })
    }
}

#[derive(Debug)]
struct Subfile {
    attributes: Attributes,
    original_length: u32,
    timestamp: u32,
    data: Vec<u8>,
}

impl Subfile {
    #[inline]
    fn load<T: ReadExt + SeekExt>(data: &mut T, header: &SubfileHeader) -> Result<Self, self::Error> {
        data.set_position(header.offset.into())?;
        Ok(Subfile {
            attributes: header.attributes,
            original_length: header.original_length,
            timestamp: header.timestamp,
            data: data.read_slice(header.length as usize)?.to_vec(),
        })
    }
}
