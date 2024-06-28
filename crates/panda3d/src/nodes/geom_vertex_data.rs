use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexData {
    pub name: String,
    /// Reference to the associated GeomVertexFormat that defines the current data
    pub format: u32,
    pub usage_hint: UsageHint,
    /// References to all GeomVertexArrayData
    pub arrays: Vec<u32>,
    pub transform_table: Option<u32>,
    pub transform_blend_table: Option<u32>,
    pub slider_table: Option<u32>,
}

impl GeomVertexData {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;

        // Cycler data
        let format = loader.read_pointer(data)?.unwrap();
        let usage_hint = UsageHint::from(data.read_u8()?);
        let num_arrays = data.read_u16()?;
        let mut arrays = Vec::new();
        for _ in 0..num_arrays {
            arrays.push(loader.read_pointer(data)?.unwrap());
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
