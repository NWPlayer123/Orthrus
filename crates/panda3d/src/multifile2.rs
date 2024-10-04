#[cfg(feature = "std")]
use std::collections::BTreeMap;
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

use bitflags::bitflags;
use orthrus_core::prelude::*;
use snafu::prelude::*;

#[derive(Debug, Snafu)]
pub enum Error {
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
            DataError::EndOfFile => Self::EndOfFile,
            _ => todo!(),
        }
    }
}

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
pub struct MultifileHeader {
    version: Version,
    scale_factor: u32,
    timestamp: u32,
}

#[expect(dead_code)]
pub struct Multifile {
    header: MultifileHeader,
    files: BTreeMap<String, Subfile>,
}

impl Multifile {
    /// Latest revision of the Multifile format. For more info, see [here](self#revisions).
    pub const CURRENT_VERSION: Version = Version { major: 1, minor: 1 };
    /// Unique identifier that tells us if we're reading a Multifile archive.
    pub const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    #[inline]
    fn read_header(data: &mut DataCursor) -> Result<MultifileHeader, self::Error> {
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

        Ok(MultifileHeader { version, scale_factor, timestamp })
    }

    #[inline]
    #[cfg(feature = "std")]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, self::Error> {
        fn inner(path: &Path) -> Result<Multifile, self::Error> {
            let data = std::fs::read(path)?;
            Multifile::load(data)
        }
        inner(path.as_ref())
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self, self::Error> {
        fn inner(input: Box<[u8]>) -> Result<Multifile, self::Error> {
            let mut data = DataCursor::new(input, Endian::Little);
            let header = Multifile::read_header(&mut data)?;

            // We operate on the assumption of an "optimized" Multifile, with all Subfile headers at the
            // start, followed by the actual data. Accumulate all headers, and then parse the data into a
            // BTreeMap to optimize cache efficiency, even if we're technically "in-memory".
            let mut headers = Vec::new();

            let mut next_index = data.read_u32()? * header.scale_factor;
            while next_index != 0 {
                let subfile = SubfileHeader::load(&mut data, header.version)?;
                headers.push(subfile);

                data.set_position(next_index as usize)?;
                next_index = data.read_u32()? * header.scale_factor;
            }

            // Now, let's actually build our sorted list of files (ideally, this will already be sorted inside
            // the Multifile)
            let mut files = BTreeMap::new();
            for mut subfile_header in headers {
                // First, let's verify that our optional parameters are valid
                // TODO: if we're on version 1.0, grab the current timestamp as a placeholder?
                if subfile_header.timestamp == 0 {
                    subfile_header.timestamp = header.timestamp;
                }
                if subfile_header.original_length == 0 {
                    subfile_header.original_length = subfile_header.length;
                }

                let subfile = Subfile::load(&mut data, &subfile_header)?;
                files.insert(subfile_header.filename, subfile);
            }

            Ok(Multifile { header, files })
        }
        inner(input.into())
    }

    #[inline]
    #[cfg(feature = "std")]
    pub fn extract_all<P: AsRef<Path>>(&mut self, output: P) -> Result<usize, self::Error> {
        fn inner(multifile: &mut Multifile, output: &PathBuf) -> Result<usize, self::Error> {
            let mut saved_files = 0;
            for subfile in &multifile.files {
                if !subfile
                    .1
                    .attributes
                    .intersects(Attributes::Signature | Attributes::Compressed | Attributes::Encrypted)
                {
                    let path = output.join(subfile.0);

                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }

                    std::fs::write(path, &subfile.1.data)?;
                    saved_files += 1;
                }
            }
            Ok(saved_files)
        }
        inner(self, &PathBuf::from(output.as_ref()))
    }

    #[inline]
    #[cfg(feature = "std")]
    pub fn extract_from_file<P: AsRef<Path>>(input: P, output: P) -> Result<usize, self::Error> {
        fn inner(input: &Path, output: &PathBuf) -> Result<usize, self::Error> {
            let input = std::fs::read(input)?;
            let mut data = DataCursor::new(input, Endian::Little);
            let header = Multifile::read_header(&mut data)?;

            // We operate on the assumption of an "optimized" Multifile, with all Subfile headers at the
            // start, followed by the actual data. Accumulate all headers, and then extract file data to
            // optimize cache efficiency, even if we're technically "in-memory".
            let mut headers = Vec::new();

            let mut next_index = data.read_u32()? * header.scale_factor;
            while next_index != 0 {
                let subfile = SubfileHeader::load(&mut data, header.version)?;
                headers.push(subfile);

                data.set_position(next_index as usize)?;
                next_index = data.read_u32()? * header.scale_factor;
            }

            // Now, let's actually extract to the filesystem
            let mut saved_files = 0;
            for mut subfile_header in headers {
                // First, let's verify that our optional parameters are valid
                // TODO: if we're on version 1.0, grab the current timestamp as a placeholder?
                // TODO: We also should probably set the timestamp in the filesystem
                if subfile_header.timestamp == 0 {
                    subfile_header.timestamp = header.timestamp;
                }
                if subfile_header.original_length == 0 {
                    subfile_header.original_length = subfile_header.length;
                }

                if !subfile_header
                    .attributes
                    .intersects(Attributes::Signature | Attributes::Compressed | Attributes::Encrypted)
                {
                    let path = output.join(subfile_header.filename);

                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }

                    data.set_position(subfile_header.offset as usize)?;

                    std::fs::write(path, data.read_slice(subfile_header.length as usize)?)?;
                    saved_files += 1;
                }
            }

            Ok(saved_files)
        }
        inner(input.as_ref(), &PathBuf::from(output.as_ref()))
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
    fn load(data: &mut DataCursor, version: Version) -> Result<Self, self::Error> {
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

#[expect(dead_code)]
struct Subfile {
    attributes: Attributes,
    original_length: u32,
    timestamp: u32,
    data: Vec<u8>,
}

impl Subfile {
    #[inline]
    fn load(data: &mut DataCursor, header: &SubfileHeader) -> Result<Self, self::Error> {
        data.set_position(header.offset as usize)?;
        Ok(Subfile {
            attributes: header.attributes,
            original_length: header.original_length,
            timestamp: header.timestamp,
            data: data.read_slice(header.length as usize)?.to_vec(),
        })
    }
}
