use super::{geom_enums::AnimationType, prelude::*};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexAnimationSpec {
    pub animation_type: AnimationType,
    pub num_transforms: u16,
    pub indexed_transforms: bool,
}

impl GeomVertexAnimationSpec {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let animation_type = AnimationType::from(data.read_u8()?);
        let num_transforms = data.read_u16()?;
        let indexed_transforms = data.read_bool()?;

        Ok(Self { animation_type, num_transforms, indexed_transforms })
    }
}

impl GraphDisplay for GeomVertexAnimationSpec {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{GeomVertexAnimationSpec|")?;
        }

        // Fields
        write!(label, "animation_type: {:?}|", self.animation_type)?;
        write!(label, "num_transforms: {:#06X}|", self.num_transforms)?;
        write!(label, "indexed_transforms: {}", self.indexed_transforms)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
