use orthrus_core::data::EndianExt;

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexArrayData {
    /// Reference to the associated GeomVertexArrayFormat used to interpret the data
    pub array_format_ref: u32,
    /// Usage hint on how often the data in question will be modified/rendered
    pub usage_hint: UsageHint,
    /// Raw vertex data, stored as a u8 array and interpreted according to the array format
    pub buffer: Vec<u8>,
}

impl Node for GeomVertexArrayData {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let array_format_ref = loader.read_pointer(data)?.unwrap();

        //Cycler data
        let usage_hint = UsageHint::from(data.read_u8()?);

        let buffer = match loader.get_minor_version() >= 8 {
            true => {
                let size = data.read_u32()?;
                let mut buffer = vec![0u8; size as usize];
                data.read_length(&mut buffer)?;
                buffer
            }
            false => {
                let _ptr_to_array = loader.read_pta_id(data)?;
                unimplemented!("I don't have any <6.8 BAM files - contact me");
            }
        };

        //TODO: byteswap if endianness doesn't match
        if data.endian() != Endian::default() {
            unimplemented!("Need to byteswap GeomVertexArrayData - non-native endianness");
        }

        Ok(Self { array_format_ref, usage_hint, buffer })
    }
}

impl GraphDisplay for GeomVertexArrayData {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{GeomVertexArrayData|")?;
        }

        // Fields
        connections.push(self.array_format_ref);
        write!(label, "usage_hint: {:?}|", self.usage_hint)?;
        // Don't try to print the buffer data, it's way too big
        write!(label, "buffer: [...]")?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
