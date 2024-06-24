use super::{geom_enums::UsageHint, prelude::*};

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct GeomVertexArrayData {
    array_format: Option<u32>,
    usage_hint: UsageHint,
    buffer: Vec<u8>,
}

impl GeomVertexArrayData {
    #[allow(unused_assignments)]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let array_format = loader.read_pointer(data)?;

        //Cycler data
        let usage_hint = UsageHint::from(data.read_u8()?);
        let mut buffer = Vec::new();
        if loader.get_minor_version() < 8 {
            let _ptr_to_array = loader.read_pta_id(data)?;
            unimplemented!("I don't have any <=6.8 BAM files - contact me");
        } else {
            let size = data.read_u32()?;
            buffer = Vec::with_capacity(size as usize);
            buffer.resize(size as usize, 0);
            data.read_length(&mut buffer)?;
        }
        Ok(Self { array_format, usage_hint, buffer })
    }
}
