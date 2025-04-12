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

impl Node for PartBundle {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
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

        Ok(Self { inner, anim_preload_ref, blend_type, anim_blend_flag, frame_blend_flag, root_transform })
    }
}

impl GraphDisplay for PartBundle {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{PartBundle|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        if let Some(reference) = self.anim_preload_ref {
            connections.push(reference);
        }
        write!(label, "|blend_type: {:?}", self.blend_type)?;
        write!(label, "|anim_blend_flag: {}", self.anim_blend_flag)?;
        write!(label, "|frame_blend_flag: {}", self.frame_blend_flag)?;
        write!(
            label,
            "|{{root_transform|{}\\n{}\\n{}\\n{}}}",
            self.root_transform.w_axis,
            self.root_transform.x_axis,
            self.root_transform.y_axis,
            self.root_transform.z_axis
        )?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
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
