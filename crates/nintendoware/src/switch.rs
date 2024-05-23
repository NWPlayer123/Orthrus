#[cfg(feature = "std")]
use std::path::Path;

use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::error::*;

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

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl Version {
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
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Default)]
struct BinaryHeader {
    magic: [u8; 4],
    byte_order: ByteOrderMark,
    version: Version,
    file_size: u32,
}

#[derive(Default, Debug)]
struct SectionInfo {
    identifier: u16,
    //2 bytes of padding to align
    offset: u32,
    size: u32,
}

#[derive(Default, Debug)]
struct SectionHeader {
    magic: [u8; 4],
    size: u32,
}

#[derive(Default, Debug)]
struct StringBlock {
    header: SectionHeader,
}

#[derive(Default, Debug)]
struct InfoBlock {
    header: SectionHeader,
}

#[derive(Default, Debug)]
struct FileBlock {
    header: SectionHeader,
}

#[derive(Default, Debug)]
/// Binary caFe Sound ARchive
pub struct BFSAR {
    header: BinaryHeader,
    sections: [SectionInfo; 3],
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
        //Initialize the data
        let mut data = DataCursor::new(input, Endian::Big);

        let mut archive = Self::default();

        archive.header = Self::read_header(&mut data)?;

        println!("{}", data.position());
        println!("{:?}", archive.header);
        
        Ok(())
    }
}
