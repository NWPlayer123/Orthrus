use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexData {
    pub name: String,
    /// Reference to the associated GeomVertexFormat that defines the current data
    pub format_ref: u32,
    pub usage_hint: UsageHint,
    /// References to all GeomVertexArrayData
    pub array_refs: Vec<u32>,
    pub transform_table_ref: Option<u32>,
    pub transform_blend_table_ref: Option<u32>,
    pub slider_table_ref: Option<u32>,
}

impl Node for GeomVertexData {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;

        // Cycler data
        let format_ref = loader.read_pointer(data)?.unwrap();
        let usage_hint = UsageHint::from(data.read_u8()?);

        let num_arrays = data.read_u16()?;
        let mut array_refs = Vec::with_capacity(num_arrays as usize);
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

impl GraphDisplay for GeomVertexData {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{GeomVertexData|")?;
        }
        // Fields
        write!(label, "name: {}|", self.name)?;
        connections.push(self.format_ref);
        write!(label, "usage_hint: {:?}", self.usage_hint)?;
        for reference in &self.array_refs {
            connections.push(*reference);
        }
        if let Some(reference) = self.transform_table_ref {
            connections.push(reference);
        }
        if let Some(reference) = self.transform_blend_table_ref {
            connections.push(reference);
        }
        if let Some(reference) = self.slider_table_ref {
            connections.push(reference);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
