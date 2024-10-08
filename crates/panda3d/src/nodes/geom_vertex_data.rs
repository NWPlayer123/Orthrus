use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct GeomVertexData {
    pub name: String, //_name
    /// Reference to the associated GeomVertexFormat that defines the current data
    pub format_ref: u32, //_format
    pub usage_hint: UsageHint, //_usage_hint
    /// References to all GeomVertexArrayData
    pub array_refs: Vec<u32>, //_arrays
    pub transform_table_ref: Option<u32>, //_transform_table
    pub transform_blend_table_ref: Option<u32>, //_transform_blend_table
    pub slider_table_ref: Option<u32>, //_slider_table
}

impl GeomVertexData {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;

        // Cycler data
        let format_ref = loader.read_pointer(data)?.unwrap();
        let usage_hint = UsageHint::from(data.read_u8()?);
        let num_arrays = data.read_u16()?;
        let mut array_refs = Vec::new();
        for _ in 0..num_arrays {
            array_refs.push(loader.read_pointer(data)?.unwrap());
        }

        let transform_table_ref = loader.read_pointer(data)?;
        let transform_blend_table_ref = loader.read_pointer(data)?;
        let slider_table_ref = loader.read_pointer(data)?;

        Ok(Self {
            name,
            format_ref,
            usage_hint,
            array_refs,
            transform_table_ref,
            transform_blend_table_ref,
            slider_table_ref,
        })
    }
}
