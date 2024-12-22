use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimBundle {
    pub inner: AnimGroup,
    pub fps: f32,
    pub num_frames: u16,
}

impl Node for AnimBundle {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = AnimGroup::create(loader, data)?;
        let fps = data.read_float()?;
        let num_frames = data.read_u16()?;
        Ok(Self { inner, fps, num_frames })
    }
}

impl GraphDisplay for AnimBundle {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{AnimBundle|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(label, "|fps: {}", self.fps)?;
        write!(label, "|num_frames: {}", self.num_frames)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for AnimBundle {
    type Target = AnimGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AnimBundle {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
