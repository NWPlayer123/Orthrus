#[cfg(feature = "std")]
use std::path::Path;

use bitflags::bitflags;
use orthrus_core::prelude::*;
use snafu::prelude::*;

/// Error conditions for when working with RARC Archives.
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
    /// Thrown if the header contains a magic number other than "RARC".
    #[snafu(display("Invalid Magic! Expected {:?}.", ResourceArchive::MAGIC))]
    InvalidMagic,
    /// Catch-all, thrown when data read differs from the known file format.
    #[snafu(display("Unexpected value encountered!"))]
    UnknownFormat,
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

bitflags! {
    #[derive(Debug, PartialEq, Default)]
    pub struct Flags: u8 {
        /// This is a file, not a directory.
        const File = 1 << 0;
        /// This is a directory, not a file.
        const Directory = 1 << 1;
        /// The file is compressed, either with Yay0 or Yaz0 compression.
        const Compressed = 1 << 2;
        /// Load this file into the Gamecube's primary RAM.
        const MainRAM = 1 << 3;
        /// Load this file into the GameCube's Audio RAM.
        const AudioRAM = 1 << 4;
        /// Load this file directlyt from the DVD when needed, instead of preloading it.
        const LoadDVD = 1 << 5;
        /// This file uses Yaz0 compression instead of Yay0 compression.
        const Yaz0 = 1 << 6;
    }
}

/// See the module [header](self#header) for more information.
pub struct Header {
    /// The size of the entire archive.
    pub file_size: u32,
    /// Offset to the start of the data header. Should always be 0x20.
    pub data_offset: u32,
    /// The size of all data, including preload and DVD data.
    pub data_size: u32,
    /// The size of all data to be preloaded into primary RAM.
    pub mram_preload_size: u32,
    /// The size of all data to be preloaded into audio RAM.
    pub aram_preload_size: u32,
}

pub struct DataHeader {
    /// The number of directories in the archive.
    pub node_count: u32,
    /// Offset to the start of the node entries.
    pub node_offset: u32,
    /// The number of files in the archive.
    pub file_count: u32,
    /// Offset to the start of the file entries.
    pub file_offset: u32,
    /// The size of the entire string table.
    pub string_tbl_size: u32,
    /// Offset to the start of the string table.
    pub string_tbl_offset: u32,
    /// This is the file ID that should be used when adding another file to the archive.
    pub next_file_id: u16,
    /// This flag means that file IDs will be synced to their index in the archive.
    pub sync_file_id: bool,
}

pub struct ResourceArchive {}

impl ResourceArchive {
    /// Unique identifier that tells us if we're reading a Resource Archive.
    pub const MAGIC: [u8; 4] = *b"RARC";

    /// Opens a file on disk, loads its contents, and parses it into a new instance, which can then
    /// be used for further operations.
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the magic number does not match a Resource
    /// Archive or [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(input: P) -> Result<Self> {
        let data = std::fs::read(input)?;
        Self::load(data)
    }

    /// Returns the metadata from a Resource Archive header.
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the magic number does not match a Resource
    /// Archive or [`UnknownFormat`](Error::UnknownFormat) if the header is larger than 0x20 in
    /// size.
    #[inline]
    fn read_header<T: ReadExt + SeekExt>(data: &mut T) -> Result<Header> {
        //Store the starting position since all offsets are relative
        let start_pos = data.position()?;

        //Read the magic and make sure we're actually parsing a Resource Archive
        let magic = data.read_slice(4)?;
        ensure!(*magic == Self::MAGIC, InvalidMagicSnafu);

        let file_size = data.read_u32()?;

        //Theoretically this can be different than 0x20 but I've only verified "standard" RARC
        let mut data_offset = data.read_u32()?;
        ensure!(data_offset == 0x20, UnknownFormatSnafu);

        //Translate it into an absolute offset
        data_offset += start_pos as u32;

        let data_size = data.read_u32()?;
        let mram_preload_size = data.read_u32()?;
        let aram_preload_size = data.read_u32()?;

        //We have 4 bytes of padding we ignore here.
        data.set_position(data_offset.into())?;

        Ok(Header {
            file_size,
            data_offset,
            data_size,
            mram_preload_size,
            aram_preload_size,
        })
    }

    /// Returns the metadata from a Resource Archive data header.
    #[inline]
    fn read_data_header<T: ReadExt + SeekExt>(data: &mut T) -> Result<DataHeader> {
        //Store the starting position since all offsets are relative
        let start_pos = data.position()?;

        //Read data
        let node_count = data.read_u32()?;
        let mut node_offset = data.read_u32()?;

        let file_count = data.read_u32()?;
        let mut file_offset = data.read_u32()?;

        let string_tbl_size = data.read_u32()?;
        let mut string_tbl_offset = data.read_u32()?;

        //Translate offsets to be absolute
        node_offset += start_pos as u32;
        file_offset += start_pos as u32;
        string_tbl_offset += start_pos as u32;

        //Read the internal flag data, this is never actually used by the game.
        let next_file_id = data.read_u16()?;
        let sync_file_id = data.read_u8()? != 0;

        //We're at 0x1A, align to 0x20
        data.set_position(node_offset.into())?;

        Ok(DataHeader {
            node_count,
            node_offset,
            file_count,
            file_offset,
            string_tbl_size,
            string_tbl_offset,
            next_file_id,
            sync_file_id,
        })
    }

    /// Loads the data from the given file and parses it into a new instance, which can then be used
    /// for further operations.
    ///
    /// # Errors
    /// Returns [`InvalidMagic`](Error::InvalidMagic) if the magic number does not match a Resource
    /// Archive or [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<Self> {
        let mut data = DataCursor::new(input, Endian::Big);
        let _header = Self::read_header(&mut data)?;
        let _data_header = Self::read_data_header(&mut data)?;

        Err(Error::EndOfFile)
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
    pub fn extract_from_path<P: AsRef<Path>>(input: P, output: P) -> Result<()> {
        let data = std::fs::read(input)?;
        Self::extract_from(&data, output)?;
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
    pub fn extract_from<P: AsRef<Path>>(input: &[u8], _output: P) -> Result<()> {
        let mut data = DataCursor::new(input, Endian::Big);
        let _header = Self::read_header(&mut data)?;
        let _data_header = Self::read_data_header(&mut data)?;

        //Now we should load a reference to the string table so we can build file/folder data to do
        //a single pass over the actual file data when writing

        /*let doc = "
        files:
          - path: \"path/to/file1\"
            flags:
                - compressed
                - encrypted
          - path: \"path/to/file2\"
            flags:
                - compressed
        ";
                use yaml_peg::{dump, node, parse, repr::RcRepr};
                let root = &parse::<RcRepr>(doc).unwrap()[0];
                let files = root.get("files").unwrap();
                for n in files.as_seq().unwrap() {
                    let path = n.get("path").unwrap();
                    let flags = n.get("flags").unwrap();
                    println!("File Path: {:?}", path.as_str().unwrap());
                    println!("{:?}", flags);
                }

                let doc = dump::<RcRepr>(
                    &[node!({
                        "files" => node!([
                            node!({
                                "path" => "path/to/file1",
                                "flags" => node!([
                                    "compressed", "encrypted"
                                ])
                            }),
                            node!({
                                "path" => "path/to/file2",
                                "flags" => node!([
                                    "compressed"
                                ])
                            }),
                        ])
                    })],
                    &[],
                );

                println!("{}", doc);*/
        Ok(())
    }
}
