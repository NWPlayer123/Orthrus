#[cfg(feature = "std")]
use std::{
    fs::File,
    io::{prelude::*, BufReader},
    path::{Path, PathBuf},
};

/// Adds support for the Resource Pack (PCK) format used by the Godot game engine.
///
/// This module is designed to assume little-endian files, as Godot will almost always be on a
/// little-endian platform, but it does have the capability to save a file as big-endian. If you
/// encounter this, please let me know!
///
/// # Format
/// The PCK format is designed to be easily parsable, and able to be embedded "inside" an executable for
/// ease of distribution. There are multiple paths to locating a PCK inside a provided file. First, it
/// will check if the file just a plain PCK by checking for the "GDPC" magic. If it doesn't find that, it
/// will try to open the file as an executable and find a section labeled "pck". If it can't find that,
/// it will check the last 4 bytes of the file. If it matches the "GDPC" magic, it will load the
/// mini-header at the end of the file to obtain the relative offset to the start of the PCK.
use orthrus_core::prelude::*;
#[allow(unused_imports)]
use orthrus_windows::pe::PortableExecutable;
use snafu::prelude::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Filesystem Error {}", source))]
    FileError { source: std::io::Error },

    /// Thrown if trying to read the file out of its current bounds.
    #[snafu(display("Reached the end of the current stream!"))]
    EndOfFile,

    /// Thrown if the header contains a magic number other than "pmf\0\n\r".
    #[snafu(display("Invalid Magic! Expected {:?}.", ResourcePack::MAGIC))]
    InvalidMagic,
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

#[allow(dead_code)]
#[derive(Debug)]
struct Header {
    pck_version: u32,
    godot_version: (u32, u32, u32),
}

#[allow(dead_code)]
#[derive(Debug)]
struct FileEntry {
    file_path: String,
    file_offset: u64,
    file_size: u64,
    md5_hash: [u8; 16],
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ResourcePack {
    header: Header,
    entries: Vec<FileEntry>,
}

impl ResourcePack {
    /// Unique identifier that tells us if we're reading a Godot PCK archive.
    pub const MAGIC: [u8; 4] = *b"GDPC";

    #[inline]
    fn read_header<T: ReadExt>(data: &mut T) -> Result<Header, self::Error> {
        let magic = data.read_exact::<4>()?;
        ensure!(magic == Self::MAGIC, InvalidMagicSnafu);

        let pck_version = data.read_u32()?;
        let godot_version = (data.read_u32()?, data.read_u32()?, data.read_u32()?);
        // TODO: these are reserved, verify they're actually zero?
        for _ in 0..16 {
            data.read_u32()?;
        }
        Ok(Header { pck_version, godot_version })
    }

    #[inline]
    #[cfg(feature = "std")]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, self::Error> {
        // Wrap this in an inner function so we can better handle generics
        fn inner(path: &Path) -> Result<ResourcePack, self::Error> {
            // We should be fine with a BufReader, this shouldn't need much seeking.
            let data = BufReader::new(File::open(path)?);
            ResourcePack::load(data)
        }
        inner(path.as_ref())
    }

    #[inline]
    pub fn load<T: Read + Seek>(input: T) -> Result<Self, self::Error> {
        //TODO: Support PE wrapper, add our cascade tree
        let mut data = DataStream::new(input, Endian::Little);
        Self::load_inner(&mut data)
    }

    /// Loads the entire `ResourcePack` metadata and returns it as an object. Used for sharing a ReadExt +
    /// SeekExt stream across multiple operations.
    ///
    /// This assumes that the input data is already at the start of a "GDPC" section, i.e. we've already
    /// parsed out any potential PE data.
    fn load_inner<T: ReadExt>(data: &mut T) -> Result<Self, self::Error> {
        // Grab the header, we need it in order to figure out which PCK version we're reading
        // TODO: support v2 and v0 archives
        let header = ResourcePack::read_header(data)?;

        // Then, let's collect all file metadata
        let file_count = data.read_u32()?;
        let mut entries = Vec::with_capacity(file_count as usize);
        for _ in 0..file_count {
            entries.push(Self::read_entry(data)?);
        }

        Ok(ResourcePack { header, entries })
    }

    pub fn extract_from_file<P: AsRef<Path>>(input: P, output: P) -> Result<usize, self::Error> {
        fn inner(input: &Path, _output: &PathBuf) -> Result<usize, self::Error> {
            // Use our existing functions to do the bulk of the loading
            let file = BufReader::new(File::open(input)?);
            let mut data = DataStream::new(file, Endian::Little);
            let mut metadata = ResourcePack::load_inner(&mut data)?;

            // In order to optimize seeking, we need to sort by file offset
            metadata.entries.sort_by_key(|entry| entry.file_offset);
            for entry in metadata.entries {
                data.set_position(entry.file_offset.into())?;
            }
            Ok(0)
        }
        inner(input.as_ref(), &PathBuf::from(output.as_ref()))
    }

    fn read_entry<T: ReadExt>(data: &mut T) -> Result<FileEntry, self::Error> {
        let string_length = data.read_u32()?;
        let file_path = data.read_string(string_length as usize)?.trim_end_matches('\0').to_owned();
        let file_offset = data.read_u64()?;
        let file_size = data.read_u64()?;
        let md5_hash = data.read_exact::<16>()?;
        Ok(FileEntry { file_path, file_offset, file_size, md5_hash })
    }
}
