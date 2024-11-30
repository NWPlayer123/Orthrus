use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::error::*;

#[derive(Debug)]
#[allow(dead_code)]
pub struct FileHeader {
    magic: [u8; 4],
    endian: [u8; 2],
    version: u16,
    file_size: u32,
    pub(crate) header_size: u16,
    block_count: u16,
}

impl FileHeader {
    #[inline]
    pub fn new<T: ReadExt>(data: &mut T, magic: [u8; 4]) -> Result<Self> {
        // Check that we got the expected magic
        let this_magic = data.read_exact()?;
        ensure!(this_magic == magic, InvalidMagicSnafu { expected: magic });

        // Obtain the current endian and change it if needed
        let endian = data.read_exact()?;
        match endian {
            [0xFE, 0xFF] => data.set_endian(Endian::Big),
            [0xFF, 0xFE] => data.set_endian(Endian::Little),
            endian => InvalidEndianSnafu { endian }.fail()?,
        }

        let version = data.read_u16()?;
        let file_size = data.read_u32()?;
        let header_size = data.read_u16()?;
        let block_count = data.read_u16()?;
        Ok(Self { magic, endian, version, file_size, header_size, block_count })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BlockHeader {
    pub magic: [u8; 4],
    pub block_size: u32,
}

impl BlockHeader {
    #[inline]
    pub fn new<T: ReadExt>(data: &mut T, magic: [u8; 4]) -> Result<Self> {
        // Check that we got the expected magic
        let this_magic = data.read_exact()?;
        ensure!(this_magic == magic, InvalidMagicSnafu { expected: magic });

        let block_size = data.read_u32()?;
        Ok(Self { magic: this_magic, block_size })
    }
}
