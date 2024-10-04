use super::geom_enums::AnimationType;
use super::prelude::*;

#[derive(Default, Debug)]
#[expect(dead_code)]
pub(crate) struct GeomVertexAnimationSpec {
    animation_type: AnimationType,
    num_transforms: u16,
    indexed_transforms: bool,
}

impl GeomVertexAnimationSpec {
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let animation_type = AnimationType::from(data.read_u8()?);
        let num_transforms = data.read_u16()?;
        let indexed_transforms = data.read_bool()?;

        Ok(Self { animation_type, num_transforms, indexed_transforms })
    }
}
