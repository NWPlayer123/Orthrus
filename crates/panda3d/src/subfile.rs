use crate::multifile::{Result, Version};
use bitflags::bitflags;
use orthrus_core::prelude::*;

#[cfg(feature = "std")]
use std::{
    fs::File,
    io::prelude::*,
    path::{Path, PathBuf},
};

bitflags! {
    #[derive(Debug, PartialEq, Default)]
    pub(crate) struct Flags: u16 {
        const Deleted = 1 << 0;
        const IndexInvalid = 1 << 1;
        const DataInvalid = 1 << 2;
        const Compressed = 1 << 3;
        const Encrypted = 1 << 4;
        const Signature = 1 << 5;
        const Text = 1 << 6;
    }
}

#[derive(Default, Debug)]
pub struct Subfile {
    pub(crate) offset: u32,
    pub(crate) length: u32,
    pub(crate) flags: Flags,
    pub(crate) timestamp: u32,
    pub(crate) filename: String,
}

impl Subfile {
    pub(crate) fn load<T>(input: &mut T, version: Version) -> Result<Self>
    where
        T: EndianRead + Read,
    {
        let offset = input.read_u32()?;
        let data_length = input.read_u32()?;
        let flags = Flags::from_bits_truncate(input.read_u16()?);

        let length = match flags.intersects(Flags::Compressed | Flags::Encrypted) {
            true => input.read_u32()?,
            false => data_length,
        };

        let timestamp = match version.minor >= 1 {
            true => input.read_u32()?,
            false => 0,
        };

        let name_length = input.read_u16()?;
        let mut filename = String::with_capacity(name_length.into());
        for _ in 0..name_length {
            filename.push((255 - input.read_u8()?) as char);
        }

        Ok(Self {
            offset,
            length,
            flags,
            timestamp,
            filename,
        })
    }

    #[cfg(feature = "std")]
    #[inline]
    pub(crate) fn write_file<P: AsRef<Path>>(&mut self,
        data: &[u8],
        output: P,
    ) -> Result<()> {
        let mut path = PathBuf::from(output.as_ref());
        path.push(&self.filename);

        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        let mut file = File::create(path)?;
        file.write_all(data)?;
        Ok(())
    }
}
