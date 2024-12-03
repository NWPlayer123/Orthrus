use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub(crate) enum DepthMode {
    Off,
    #[default]
    On,
}

#[derive(Debug, Default)]
pub(crate) struct DepthWriteAttrib {
    pub mode: DepthMode,
}

impl Node for DepthWriteAttrib {
    #[inline]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        Ok(Self { mode: DepthMode::from(data.read_u8()?) })
    }
}

impl Deref for DepthWriteAttrib {
    type Target = DepthMode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.mode
    }
}

impl DerefMut for DepthWriteAttrib {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mode
    }
}
