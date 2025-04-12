use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct Geom {
    /// Reference to the associated GeomVertexData
    pub data_ref: u32,
    /// References to all GeomPrimitive data
    pub primitive_refs: Vec<u32>,
    pub primitive_type: PrimitiveType,
    pub shade_model: ShadeModel,
    pub geom_rendering: GeomRendering,
    pub bounds_type: BoundsType,
}

impl Node for Geom {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let data_ref = loader.read_pointer(data)?.unwrap();

        let num_primitives = data.read_u16()?;
        let mut primitive_refs = Vec::with_capacity(num_primitives as usize);
        for _ in 0..num_primitives {
            primitive_refs.push(loader.read_pointer(data)?.unwrap());
        }

        let primitive_type = PrimitiveType::from(data.read_u8()?);
        let shade_model = ShadeModel::from(data.read_u8()?);

        //TODO: if this ever gets removed, we should re-derive this bitfield using reset_geom_rendering()
        let geom_rendering = GeomRendering::from_bits_truncate(data.read_u16()?.into());

        let bounds_type = match loader.get_minor_version() >= 19 {
            true => BoundsType::from(data.read_u8()?),
            false => BoundsType::Default,
        };

        Ok(Self { data_ref, primitive_refs, primitive_type, shade_model, geom_rendering, bounds_type })
    }
}

impl GraphDisplay for Geom {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{Geom|")?;
        }

        // Fields
        connections.push(self.data_ref);
        for reference in &self.primitive_refs {
            connections.push(*reference);
        }
        write!(label, "primitive_type: {:?}|", self.primitive_type)?;
        write!(label, "shade_model: {:?}|", self.shade_model)?;
        write!(label, "geom_rendering: {:?}|", self.geom_rendering)?;
        write!(label, "bounds_type: {:?}", self.bounds_type)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
