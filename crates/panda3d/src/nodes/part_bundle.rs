use core::ops::{Deref, DerefMut};

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
    pub inner: PartGroup,
    pub anim_preload_ref: Option<u32>,
    pub blend_type: BlendType,
    pub anim_blend_flag: bool,
    pub frame_blend_flag: bool,
    pub root_transform: Mat4,
}

impl PartBundle {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PartGroup::create(loader, data)?;

        let anim_preload_ref = match loader.get_minor_version() >= 17 {
            true => loader.read_pointer(data)?,
            false => None,
        };

        // Cycler Data
        if loader.get_minor_version() < 10 {
            unimplemented!("I don't have any BAM files this old - contact me");
        }
        let blend_type = BlendType::from(data.read_u8()?);
        let anim_blend_flag = data.read_bool()?;
        let frame_blend_flag = data.read_bool()?;
        let root_transform = Mat4::read(data)?;

        if loader.get_minor_version() == 11 {
            unimplemented!("We need to handle _modifies_anim_bundles in PartBundle for this file");
        }

        Ok(Self {
            inner,
            anim_preload_ref,
            blend_type,
            anim_blend_flag,
            frame_blend_flag,
            root_transform,
        })
    }
}

impl Deref for PartBundle {
    type Target = PartGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PartBundle {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
