use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum BlendType {
    Linear,
    #[default]
    NormalizedLinear,
    Componentwise,
    ComponentwiseQuat,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct PartBundle {
    pub group: PartGroup,
    anim_preload_ref: Option<u32>,
    blend_type: BlendType,
    anim_blend_flag: bool,
    frame_blend_flag: bool,
    root_transform: Mat4,
}

impl PartBundle {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let group = PartGroup::create(loader, data)?;
        let anim_preload_ref = match loader.get_minor_version() >= 17 {
            true => loader.read_pointer(data)?,
            false => None,
        };
        if loader.get_minor_version() < 10 {
            panic!("I don't have any BAM files this old - contact me");
        }
        let blend_type = BlendType::from(data.read_u8()?);
        let anim_blend_flag = data.read_bool()?;
        let frame_blend_flag = data.read_bool()?;
        let root_transform = Mat4::read(data)?;
        Ok(Self {
            group,
            anim_preload_ref,
            blend_type,
            anim_blend_flag,
            frame_blend_flag,
            root_transform,
        })
    }
}
