use super::{geom_enums::UsageHint, prelude::*};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexData {
    name: String,
    format: Option<u32>,
    usage_hint: UsageHint,
    arrays: Vec<Option<u32>>,
    transform_table: Option<u32>,
    transform_blend_table: Option<u32>,
    slider_table: Option<u32>,
}

impl GeomVertexData {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;

        // Cycler data
        let format = loader.read_pointer(data)?;
        let usage_hint = UsageHint::from(data.read_u8()?);
        let num_arrays = data.read_u16()?;
        let mut arrays = Vec::new();
        for _ in 0..num_arrays {
            arrays.push(loader.read_pointer(data)?);
        }

        let transform_table = loader.read_pointer(data)?;
        let transform_blend_table = loader.read_pointer(data)?;
        let slider_table = loader.read_pointer(data)?;

        Ok(Self {
            name,
            format,
            usage_hint,
            arrays,
            transform_table,
            transform_blend_table,
            slider_table,
        })
    }
}
