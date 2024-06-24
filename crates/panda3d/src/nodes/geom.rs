use super::bounding_volume::BoundsType;
use super::geom_enums::*;
use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct Geom {
    data_ptr: Option<u32>,
    primitives: Vec<Option<u32>>,
    primitive_type: PrimitiveType,
    shade_model: ShadeModel,
    geom_rendering: u16,
    bounds_type: BoundsType,
}

impl Geom {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let data_ptr = loader.read_pointer(data)?;
        let num_primitives = data.read_u16()?;
        let mut primitives = Vec::new();
        for _ in 0..num_primitives {
            primitives.push(loader.read_pointer(data)?);
        }
        let primitive_type = PrimitiveType::from(data.read_u8()?);
        let shade_model = ShadeModel::from(data.read_u8()?);
        //TODO: if this ever gets removed
        let geom_rendering = data.read_u16()?;
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
