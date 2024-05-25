use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::path::Path;

use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::error::*;

trait Read {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self>
    where
        Self: Sized;
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteOrderMark(u16);

#[allow(non_upper_case_globals)]
impl ByteOrderMark {
    pub const Big: ByteOrderMark = ByteOrderMark(0xFEFF);
    pub const Little: ByteOrderMark = ByteOrderMark(0xFFFE);
}

impl Default for ByteOrderMark {
    #[cfg(target_endian = "little")]
    #[inline]
    fn default() -> Self {
        ByteOrderMark::Little
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn default() -> Self {
        ByteOrderMark::Big
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl Read for Version {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut version = Self::default();
        version.major = data.read_u8()?;
        version.minor = data.read_u8()?;
        version.patch = data.read_u8()?;
        //This should always be zero, but I'm not going to enforce an assert here
        let _align = data.read_u8()?;
        Ok(version)
    }
}

impl core::fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct BinaryHeader {
    magic: [u8; 4],
    byte_order: ByteOrderMark,
    size: u16,
    version: Version,
    file_size: u32,
    num_sections: u16,
    //padding: [u8; 2]
}

impl Read for BinaryHeader {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Create a header, so we can copy in its magic
        let mut header = Self::default();

        // Read in the magic
        data.read_length(&mut header.magic)?;

        // Read the Byte Order Mark and use it to update our endianness
        header.byte_order = ByteOrderMark(data.read_u16()?);
        let endian = match header.byte_order {
            ByteOrderMark::Little => Endian::Little,
            ByteOrderMark::Big => Endian::Big,
            _ => InvalidDataSnafu { reason: "Invalid Byte Order Mark!" }.fail()?,
        };
        data.set_endian(endian);

        //Read the rest of the data
        header.size = data.read_u16()?;
        header.version = Version::read(data)?;
        header.file_size = data.read_u32()?;
        header.num_sections = data.read_u16()?;
        data.set_position(data.position() + 2); //Skip alignment
        
        Ok(header)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct SizedReference {
    identifier: u16,
    //2 bytes of padding to align
    offset: u32,
    size: u32,
}

impl Read for SizedReference {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let identifier = data.read_u16()?;
        data.seek(SeekFrom::Current(2))?;
        let offset = data.read_u32()?;
        let size = data.read_u32()?;
        Ok(Self { identifier, offset, size })
    }
}

#[derive(Default, Debug)]
struct Reference {
    identifier: u16,
    //2 bytes of padding to align
    offset: u32,
}

impl Read for Reference {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let identifier = data.read_u16()?;
        data.seek(SeekFrom::Current(2))?;
        let offset = data.read_u32()?;
        Ok(Self { identifier, offset })
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct SectionHeader {
    magic: [u8; 4],
    size: u32,
}

impl Read for SectionHeader {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut header = SectionHeader::default();
        data.read_length(&mut header.magic)?;
        header.size = data.read_u32()?;
        Ok(header)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct Table<V: Read> {
    _marker: PhantomData<V>
}

impl<V: Read> Table<V> {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Vec<V>> {
        let count = data.read_u32()?;

        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            values.push(V::read(data)?);
        }

        Ok(values)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct PatriciaNode {
    flags: u16,
    search_index: u16,
    left_index: u32,
    right_index: u32,
    string_id: u32,
    item_id: u32,
}

impl Read for PatriciaNode {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        Ok(Self {
            flags: data.read_u16()?,
            search_index: data.read_u16()?,
            left_index: data.read_u32()?,
            right_index: data.read_u32()?,
            string_id: data.read_u32()?,
            item_id: data.read_u32()?,
        })
    }
}

impl Default for PatriciaNode {
    fn default() -> Self {
        Self {
            flags: 0,
            search_index: 0xFFFF,
            left_index: 0xFFFFFFFF,
            right_index: 0xFFFFFFFF,
            string_id: 0xFFFFFFFF,
            item_id: 0xFFFFFFFF,
        }
    }
}

#[derive(Default, Debug)]
struct PatriciaTree {
    root_index: u32,
    nodes: Vec<PatriciaNode>,
}

impl PatriciaTree {
    fn get_node(&self, string: String) -> Result<&PatriciaNode> {
        let mut node = self.nodes.get(self.root_index as usize).ok_or(Error::NodeNotFound)?;
        let bytes = string.as_bytes();

        // Loop as long as we haven't hit a leaf node
        while (node.flags & 1) == 0 {
            // Separate out the string position and the bit location
            let pos = (node.search_index >> 3) as usize;
            let bit = (node.search_index & 7) as usize;

            let node_index;
            if (bytes[pos] & (1 << (7 - bit))) == 1 {
                node_index = node.right_index as usize;
            } else {
                node_index = node.left_index as usize;
            }
            node = self.nodes.get(node_index).ok_or(Error::NodeNotFound)?;
        }

        Ok(node)
    }
}

impl Read for PatriciaTree {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut tree = Self::default();

        // First, get the root index
        tree.root_index = data.read_u32()?;

        // Then, we can load in the node table
        tree.nodes = Table::read(data)?;

        Ok(tree)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct StringBlock {
    table: Vec<String>,
    tree: PatriciaTree,
}

impl StringBlock {
    /// Unique identifier that tells us if we're reading a String Block.
    pub const MAGIC: [u8; 4] = *b"STRG";

    fn read_string_table<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Vec<String>> {
        // Store relative position
        let offset = data.position();

        // Read in the reference table
        let references: Vec<SizedReference> = Table::read(data)?;

        // Then we can process all strings, pre-allocate since we know the count ahead of time
        let mut strings = Vec::with_capacity(references.len() as usize);
        for reference in &references {
            // Go to that position in the string blob
            data.set_position(offset + reference.offset as usize);

            // Read the string and store it, includes the trailing \0
            let string = data.get_slice(reference.size as usize)?.to_vec();
            strings.push(String::from_utf8(string).map_err(|_| data::Error::InvalidUtf8)?);
        }

        Ok(strings)
    }
}

impl Read for StringBlock {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Store relative position
        let offset = data.position();

        // Read both sections
        let mut sections: [Reference; 2] = Default::default();

        for section in &mut sections {
            *section = Reference::read(data)?;
        }

        // Then process each section
        let mut strings = Self::default();

        for section in &mut sections {
            data.set_position(offset + section.offset as usize);
            match section.identifier {
                0x2400 => {
                    // String Table
                    strings.table = Self::read_string_table(data)?;
                }
                0x2401 => {
                    // Patricia Tree
                    strings.tree = PatriciaTree::read(data)?;
                }
                _ => InvalidDataSnafu { reason: "Unexpected String Section Identifier!" }.fail()?,
            }
        }

        Ok(strings)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct SoundInfo {}

impl SoundInfo {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        Ok(Self {})
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct InfoBlock {}

impl InfoBlock {
    /// Unique identifier that tells us if we're reading an Info Block.
    pub const MAGIC: [u8; 4] = *b"INFO";

    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Store relative position
        let offset = data.position();

        // Read all references
        let mut sections: [Reference; 8] = Default::default();
        for section in &mut sections {
            *section = Reference::read(data)?;
        }

        for section in &mut sections {
            data.set_position(offset + section.offset as usize);
            match section.identifier {
                0x2100 => {
                    // Sound Info
                    let sound_info = SoundInfo::read(data)?;
                }
                0x2101 => {
                    // Bank Info
                }
                0x2102 => {
                    // Player Info
                }
                0x2103 => {
                    // Wave Archive Info
                }
                0x2104 => {
                    // Sound Group Info
                }
                0x2105 => {
                    // Group Info
                }
                0x2106 => {
                    // File Info
                }
                0x220B => {
                    // Sound Archive Player Info
                }
                _ => InvalidDataSnafu { reason: "Unexpected Info Section Identifier!" }.fail()?,
            }
        }

        Ok(Self {})
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct FileBlock {
    header: SectionHeader,
}

impl FileBlock {
    /// Unique identifier that tells us if we're reading a File Block.
    pub const MAGIC: [u8; 4] = *b"FILE";
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
/// Binary caFe Sound ARchive
pub struct BFSAR {
    header: BinaryHeader,
    strings: StringBlock,
    info: InfoBlock,
    files: FileBlock,
}

impl BFSAR {
    /// Unique identifier that tells us if we're reading a Sound Archive.
    pub const MAGIC: [u8; 4] = *b"FSAR";

    #[inline]
    #[allow(dead_code)]
    fn read_header<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<BinaryHeader> {
        // Read the header
        let header = BinaryHeader::read(data)?;

        //Now we need to verify that it's what we actually expected
        ensure!(
            header.magic == Self::MAGIC,
            InvalidMagicSnafu { expected: Self::MAGIC }
        );

        ensure!(
            header.size == 0x40,
            InvalidDataSnafu { reason: "Header size must be 0x40!" }
        );
        
        ensure!(
            data.len() == header.file_size as usize,
            InvalidDataSnafu { reason: "Unexpected file size!" }
        );

        ensure!(
            header.num_sections == 3,
            InvalidDataSnafu { reason: "Unexpected section count!" }
        );

        Ok(header)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(input: P) -> Result<()> {
        let data = std::fs::read(input)?;
        Self::load(data)
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<()> {
        // Initialize the data
        let mut data = DataCursor::new(input, Endian::Big);

        // Start creating our return struct
        let mut archive = Self::default();

        // Read the file header
        archive.header = Self::read_header(&mut data)?;

        // Read the references to all sections
        let mut sections: [SizedReference; 3] = Default::default();
        for n in 0..sections.len() {
            sections[n] = SizedReference::read(&mut data)?;
        }

        // Align to a 32-byte boundary
        data.set_position((data.position() + 31) & !31);

        // Then read all the section data
        for section in &sections {
            data.set_position(section.offset as usize);

            let section = SectionHeader::read(&mut data)?;
            match section.magic {
                StringBlock::MAGIC => {
                    archive.strings = StringBlock::read(&mut data)?;
                }
                InfoBlock::MAGIC => {
                    archive.info = InfoBlock::read(&mut data)?;
                }
                FileBlock::MAGIC => {}
                _ => InvalidDataSnafu { reason: "Unexpected BFSAR Section!" }.fail()?,
            }
        }

        Ok(())
    }
}
