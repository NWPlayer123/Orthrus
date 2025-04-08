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

impl DepthWriteAttrib {
    #[inline]
    pub fn depth_write_enabled(&self) -> bool {
        match self.mode {
            DepthMode::Off => false,
            DepthMode::On => true,
        }
    }
}

impl Node for DepthWriteAttrib {
    #[inline]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { mode: DepthMode::from(data.read_u8()?) })
    }
}

impl GraphDisplay for DepthWriteAttrib {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        write!(label, "{{DepthWriteAttrib|")?;

        // Fields
        write!(label, "mode: {:?}", self.mode)?;

        // Footer
        write!(label, "}}")?;
        Ok(())
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
