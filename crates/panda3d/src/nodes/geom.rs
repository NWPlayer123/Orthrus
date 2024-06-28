use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct Geom {
    /// Reference to the associated GeomVertexData
    pub data_ptr: u32,
    /// References to all GeomPrimitive data
    pub primitives: Vec<u32>,
    pub primitive_type: PrimitiveType,
    pub shade_model: ShadeModel,
    pub geom_rendering: GeomRendering,
    pub bounds_type: BoundsType,
}

impl Geom {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let data_ptr = loader.read_pointer(data)?.unwrap();

        let num_primitives = data.read_u16()?;
        let mut primitives = Vec::with_capacity(num_primitives as usize);
        for _ in 0..num_primitives {
            primitives.push(loader.read_pointer(data)?.unwrap());
        }

        let primitive_type = PrimitiveType::from(data.read_u8()?);
        let shade_model = ShadeModel::from(data.read_u8()?);

        //TODO: if this ever gets removed, we should re-derive this bitfield using
        // reset_geom_rendering()
        let geom_rendering = GeomRendering::from_bits_truncate(data.read_u16()? as u32);

        let bounds_type = match loader.get_minor_version() >= 19 {
            true => BoundsType::from(data.read_u8()?),
            false => BoundsType::Default,
        };

        Ok(Self {
            data_ptr,
            primitives,
            primitive_type,
            shade_model,
            geom_rendering,
            bounds_type,
        })
    }
}
