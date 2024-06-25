use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexArrayData {
    /// Reference to the associated GeomVertexArrayFormat used to interpret the data
    array_format: u32,
    /// Usage hint on how often the data in question will be modified/rendered
    usage_hint: UsageHint,
    /// Raw vertex data, stored as a u8 array and interpreted according to the array format
    buffer: Vec<u8>,
}

impl GeomVertexArrayData {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let array_format = loader.read_pointer(data)?.unwrap();

        //Cycler data
        let usage_hint = UsageHint::from(data.read_u8()?);
        let mut buffer: Vec<u8>;
        //TODO: match statement? or fix the PTA_uchar
        if loader.get_minor_version() >= 8 {
            let size = data.read_u32()?;
            buffer = vec![0; size as usize];
            data.read_length(&mut buffer)?;
        } else {
            let _ptr_to_array = loader.read_pta_id(data)?;
            unimplemented!("I don't have any <6.8 BAM files - contact me");
        }
        Ok(Self { array_format, usage_hint, buffer })
    }
}
