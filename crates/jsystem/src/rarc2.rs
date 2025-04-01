use std::ffi::CString;
#[cfg(feature = "std")]
use std::{fs::File, io::BufReader, path::Path};

use bitflags::bitflags;
use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::prelude::*;

#[derive(Debug)]
pub struct ResourceArchive {}

impl ResourceArchive {
    /// Unique identifier that tells us if we're reading a Resource Archive.
    pub const MAGIC: [u8; 4] = *b"RARC";

    /// Opens a file on disk, loads its contents, and parses it into a new `ResourceArchive` instance. The
    /// instance can then be used for further operations.
    #[inline]
    #[cfg(feature = "std")]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let data = BufReader::new(File::open(path)?);
        Self::load(data)
    }

    #[inline]
    pub fn load<T: IntoDataStream>(input: T) -> Result<Self, Error> {
        let mut data = input.into_stream(Endian::Big);
        let header = Header::new(&mut data)?;
        println!("{header:?}");
        let data_header = DataHeader::new(&mut data)?;
        println!("{data_header:?}");
        let mut directory_nodes = Vec::with_capacity(data_header.directory_count as usize);
        for _ in 0..data_header.directory_count {
            let directory = DirectoryNode::new(&mut data)?;
            //println!("{directory:?}");
            directory_nodes.push(directory);
        }
        let mut file_nodes = Vec::with_capacity(data_header.file_count as usize);
        for _ in 0..data_header.file_count {
            let file = FileNode::new(&mut data)?;
            //println!("{file:?}");
            file_nodes.push(file);
        }
        // The String Table is 0x10 aligned, so we need to make sure we are too
        data.set_position(0x20 + u64::from(data_header.string_table_offset))?;
        let string_table = data.read_slice(data_header.string_table_size as usize)?;
        for directory in directory_nodes {
            let end = string_table[directory.string_offset as usize..]
                .iter()
                .position(|&b| b == 0)
                .map(|pos| pos + directory.string_offset as usize)
                .unwrap();
            println!(
                "{:?}:",
                CString::new(&string_table[directory.string_offset as usize..end]).unwrap()
            );
            println!("{directory:?}");
        }
        println!();
        for file in file_nodes {
            let end = string_table[file.string_offset as usize..]
                .iter()
                .position(|&b| b == 0)
                .map(|pos| pos + file.string_offset as usize)
                .unwrap();
            println!(
                "{:?}:",
                CString::new(&string_table[file.string_offset as usize..end]).unwrap()
            );
            println!("{file:?}");
        }
        Ok(Self {})
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Header {
    /// "RARC" magic, used to determine endianness
    magic: [u8; 4],
    /// Size of the entire file
    file_size: u32,
    /// Relative to the start of the data header
    data_offset: u32,
    /// Size of all stored file data
    data_size: u32,
    /// Size of all files that get loaded into MRAM
    mram_data_size: u32,
    /// Size of all files that get loaded into ARAM
    aram_data_size: u32,
}

impl Header {
    #[inline]
    fn new<T: ReadExt + SeekExt>(data: &mut T) -> Result<Self, Error> {
        // Load implicitly big endian magic, check to see if we need to swap endians
        let magic = data.read_exact::<4>()?;
        match &magic {
            b"RARC" => data.set_endian(Endian::Big),
            b"CRAR" => data.set_endian(Endian::Little),
            _ => InvalidMagicSnafu { expected: ResourceArchive::MAGIC }.fail()?,
        }

        let file_size = data.read_u32()?;
        ensure!(
            data.read_u32()? == 0x20,
            InvalidDataSnafu { position: data.position()? - 4, reason: "Header Size Must Be 0x20" }
        );
        let data_offset = data.read_u32()?;
        let data_size = data.read_u32()?;
        let mram_data_size = data.read_u32()?;
        let aram_data_size = data.read_u32()?;
        ensure!(
            data.read_u32()? == 0,
            InvalidDataSnafu { position: data.position()? - 4, reason: "This padding should be zero" }
        );

        Ok(Self {
            magic,
            file_size,
            data_offset,
            data_size,
            mram_data_size,
            aram_data_size,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct DataHeader {
    /// Number of Directory nodes
    directory_count: u32,
    /// Number of File nodes
    file_count: u32,
    /// Offset to the File Nodes, from the end of the [`Header`]
    file_offset: u32,
    /// Size of the String Table
    string_table_size: u32,
    /// Offset to the String Table, from the end of the [`Header`]
    string_table_offset: u32,
    /// Next File Index
    next_file_index: u16,
    /// Keeps File IDs Synced
    sync_file_ids: bool,
}

impl DataHeader {
    #[inline]
    fn new<T: ReadExt + SeekExt>(data: &mut T) -> Result<Self, Error> {
        let directory_count = data.read_u32()?;
        ensure!(
            data.read_u32()? == 0x20,
            InvalidDataSnafu {
                position: data.position()? - 4,
                reason: "Directory Offset Must Be 0x20"
            }
        );
        let file_count = data.read_u32()?;
        let file_offset = data.read_u32()?;
        let string_table_size = data.read_u32()?;
        let string_table_offset = data.read_u32()?;
        let next_file_index = data.read_u16()?;
        let sync_file_ids = data.read_u8()? != 0;
        ensure!(
            data.read_exact::<5>()? == [0u8; 5],
            InvalidDataSnafu { position: data.position()? - 5, reason: "Padding should be zero" }
        );

        Ok(Self {
            directory_count,
            file_count,
            file_offset,
            string_table_size,
            string_table_offset,
            next_file_index,
            sync_file_ids,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct DirectoryNode {
    /// First 4 characters of Directory name, in all caps (padded to 4 bytes with spaces)
    directory_name: [u8; 4],
    /// Offset to Directory name in String Table
    string_offset: u32,
    /// Hash of Directory Name
    name_hash: u16,
    /// Number of File Nodes in this directory
    file_count: u16,
    /// Offset to first File Node from the start of that section
    file_node_offset: u32,
}

impl DirectoryNode {
    #[inline]
    fn new<T: ReadExt>(data: &mut T) -> Result<Self, Error> {
        let directory_name = data.read_exact::<4>()?;
        let string_offset = data.read_u32()?;
        let name_hash = data.read_u16()?;
        let file_count = data.read_u16()?;
        let file_node_offset = data.read_u32()?;
        Ok(Self { directory_name, string_offset, name_hash, file_count, file_node_offset })
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct Attributes: u8 {
        const FILE = 1 << 0;
        const DIRECTORY = 1 << 1;
        const COMPRESSED = 1 << 2;
        const LOAD_MRAM = 1 << 4;
        const LOAD_ARAM = 1 << 5;
        const LOAD_DVD = 1 << 6;
        const YAZ0_COMPRESS = 1 << 7;
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FileNode {
    /// Node Index (or 0xFFFF if Directory)
    node_index: u16,
    /// Hash of File Name
    node_hash: u16,
    /// File Attributes
    attributes: Attributes,
    /// File Name Offset in String Table
    string_offset: u16,
    /// DirectoryNode Index if Directory, File Data Offset if File
    node_offset: u32,
    /// 0x10 if Directory, File Size if File
    node_size: u32,
}

impl FileNode {
    fn new<T: ReadExt + SeekExt>(data: &mut T) -> Result<Self, Error> {
        let node_index = data.read_u16()?;
        let node_hash = data.read_u16()?;
        let attributes = match Attributes::from_bits(data.read_u8()?) {
            Some(attributes) => attributes,
            None => InvalidDataSnafu { position: data.position()? - 1, reason: "Unknown Attributes Set" }
                .fail()?,
        };
        ensure!(
            data.read_u8()? == 0,
            InvalidDataSnafu { position: data.position()? - 1, reason: "Padding Should Be Zero" }
        );
        let string_offset = data.read_u16()?;
        let node_offset = data.read_u32()?;
        let node_size = data.read_u32()?;
        ensure!(
            data.read_u32()? == 0,
            InvalidDataSnafu { position: data.position()? - 4, reason: "Padding Should Be Zero" }
        );

        if attributes.contains(Attributes::DIRECTORY) {
            ensure!(
                node_index == 0xFFFF,
                InvalidDataSnafu {
                    position: data.position()? - 0x10,
                    reason: "Invalid Directory Node Index"
                }
            );
            ensure!(
                node_size == 0x10,
                InvalidDataSnafu {
                    position: data.position()? - 0x10,
                    reason: "Directory Size Should Be 0x10"
                }
            );
        }

        Ok(Self {
            node_index,
            node_hash,
            attributes,
            string_offset,
            node_offset,
            node_size,
        })
    }
}
