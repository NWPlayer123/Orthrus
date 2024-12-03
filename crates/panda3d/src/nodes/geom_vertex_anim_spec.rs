use super::geom_enums::AnimationType;
use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexAnimationSpec {
    pub animation_type: AnimationType,
    pub num_transforms: u16,
    pub indexed_transforms: bool,
}

impl GeomVertexAnimationSpec {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        let animation_type = AnimationType::from(data.read_u8()?);
        let num_transforms = data.read_u16()?;
        let indexed_transforms = data.read_bool()?;

        Ok(Self { animation_type, num_transforms, indexed_transforms })
    }
}
