use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum AnimationType {
    #[default]
    /// No vertex animation performed.
    None,
    /// Animations are processed on the CPU through Panda3D.
    Panda,
    /// Animations are hardware-accelerated on the GPU.
    Hardware,
}

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct GeomVertexAnimationSpec {
    animation_type: AnimationType,
    num_transforms: u16,
    indexed_transforms: bool,
}

impl GeomVertexAnimationSpec {
    fn _create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let animation_type = AnimationType::from(data.read_u8()?);
        let num_transforms = data.read_u16()?;
        let indexed_transforms = data.read_bool()?;

        Ok(Self {
            animation_type,
            num_transforms,
            indexed_transforms,
        })
    }
}
