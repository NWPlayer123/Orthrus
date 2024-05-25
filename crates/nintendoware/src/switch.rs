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
    //header_size: u16, should be known at compile time
    version: Version,
    file_size: u32,
    //num_sections: u16, should be known at compile time
    //pasdding: [u8; 2]
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct Reference {
    identifier: u16,
    //2 bytes of padding to align
    offset: u32,
    size: u32,
}

impl Reference {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let identifier = data.read_u16()?;
        data.seek(SeekFrom::Current(2))?;
        let offset = data.read_u32()?;
        let size = data.read_u32()?;
        Ok(Self { identifier, offset, size })
    }

    fn read_no_size<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let identifier = data.read_u16()?;
        data.seek(SeekFrom::Current(2))?;
        let offset = data.read_u32()?;
        Ok(Self { identifier, offset, size: 0 })
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct SectionHeader {
    magic: [u8; 4],
    size: u32,
}

impl SectionHeader {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut header = SectionHeader::default();

        data.read_length(&mut header.magic)?;

        header.size = data.read_u32()?;

        Ok(header)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct Table<V: Read> {
    values: Vec<V>,
}

impl<V: Read> Table<V> {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let count = data.read_u32()?;
        let mut values = Vec::with_capacity(count as usize);

        for value in &mut values {
            *value = V::read(data)?;
        }
        Ok(Self { values })
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct PatriciaNode {
    flags: u16,
    search_index: u16,
    left_index: u32,
    right_index: u32,
    string_id: u32,
    item_id: u32,
}

impl PatriciaNode {
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

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct StringBlock {
    strings: Vec<String>,
}

impl StringBlock {
    /// Unique identifier that tells us if we're reading a String Block.
    pub const MAGIC: [u8; 4] = *b"STRG";

    fn read_string_table<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Vec<String>> {
        // Store relative position
        let offset = data.position();

        // Read the number of strings in the table
        let string_count = data.read_u32()?;

        //Now let's read all references sequentially to help with caching
        let mut references = Vec::with_capacity(string_count as usize);
        for _ in 0..string_count {
            references.push(Reference::read(data)?);
        }

        //Then we can process all strings and store them, pre-allocate since we know the count
        // ahead of time
        let mut strings = Vec::with_capacity(string_count as usize);
        for n in 0..string_count {
            // Go to that position in the string blob
            let reference = &references[n as usize];
            data.set_position(offset + reference.offset as usize);

            // Read the string and store it, includes the trailing \0
            let string = data.get_slice(reference.size as usize)?.to_vec();
            strings.push(String::from_utf8(string).map_err(|_| data::Error::InvalidUtf8)?);
        }

        Ok(strings)
    }

    fn read_patricia_tree<T: DataCursorTrait + EndianRead>(
        data: &mut T,
    ) -> Result<Vec<PatriciaNode>> {
        // Get the root index
        let root_index = data.read_u32()?;

        // Now get the number of entries
        let node_count = data.read_u32()?;

        let mut nodes = Vec::with_capacity(node_count as usize);

        for _ in 0..node_count {
            let node = PatriciaNode::read(data)?;
            /*if node.flags == 1 { //Leaf node
                println!("String: {}, Item: {}", node.string_id, node.item_id);
            }
            else {
                println!("Position: {}, Bit: {}, Left: {}, Right: {}", node.search_index >> 3, node.search_index & 7, node.left_index, node.right_index);
            }*/
            nodes.push(node);
        }

        let lookup = *b"BGM_BTL_Boss_S5_Win\0";

        let mut node = nodes.get(root_index as usize).expect("No root node!");
        println!("{:?}", node);

        let length = lookup.len();

        while (node.flags & 1) == 0 {
            let pos = node.search_index >> 3;
            let bit = node.search_index & 7;
            println!("pos: {}, bit: {}", pos, bit);
            let node_index;
            if (lookup[pos as usize] & (1 << (7 - bit))) == 1 {
                node_index = node.right_index;
            } else {
                node_index = node.left_index;
            }
            node = nodes.get(node_index as usize).expect("Need a valid node!");
        }
        println!("{:?}", node);

        Ok(nodes)
    }

    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Store relative position
        let offset = data.position();

        // Read both sections
        let mut sections: [Reference; 2] = Default::default();

        for section in &mut sections {
            *section = Reference::read_no_size(data)?;
        }

        // Then process each section
        let mut strings: Vec<String> = Default::default();
        for section in &mut sections {
            data.set_position(offset + section.offset as usize);
            match section.identifier {
                0x2400 => {
                    // String Table
                    strings = Self::read_string_table(data)?;
                }
                0x2401 => {
                    // Patricia Tree
                    let tree = Self::read_patricia_tree(data)?;
                }
                _ => InvalidDataSnafu { reason: "Unexpected String Section Identifier!" }.fail()?,
            }
        }

        Ok(Self { strings })
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
            *section = Reference::read_no_size(data)?;
            println!("{:?} {:X}", *section, section.identifier);
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

        println!("{:?}", sections);
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
        //We start by verifying the magic and BOM, so we can update DataCursor with the right
        // endian
        let mut header = BinaryHeader::default();

        data.read_length(&mut header.magic)?;
        ensure!(
            header.magic == Self::MAGIC,
            InvalidMagicSnafu { expected: Self::MAGIC }
        );

        header.byte_order = ByteOrderMark(data.read_u16()?);
        let endian = match header.byte_order {
            ByteOrderMark::Little => Endian::Little,
            ByteOrderMark::Big => Endian::Big,
            _ => InvalidDataSnafu { reason: "Invalid Byte Order Mark!" }.fail()?,
        };
        data.set_endian(endian);

        let header_size = data.read_u16()?;
        ensure!(
            header_size == 0x40,
            InvalidDataSnafu { reason: "Header size must be 0x40!" }
        );

        //Read the rest of the data
        header.version = Version::read(data)?;
        header.file_size = data.read_u32()?;
        let section_count = data.read_u16()?;
        data.set_position(data.position() + 2); //Skip alignment

        //Now verify that the header is sane
        ensure!(
            data.len() == header.file_size as usize,
            InvalidDataSnafu { reason: "Unexpected file size!" }
        );
        ensure!(
            section_count == 3,
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
        let mut sections: [Reference; 3] = Default::default();
        for n in 0..3 {
            sections[n] = Reference::read(&mut data)?;
        }

        // Align to a 32-byte boundary
        data.set_position((data.position() + 31) & !31);

        // Then read all the section data
        for n in 0..3 {
            data.set_position(sections[n].offset as usize);

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
