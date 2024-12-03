use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct MovingPartBase {
    pub inner: PartGroup,
    pub forced_channel_ref: Option<u32>,
}

impl MovingPartBase {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PartGroup::create(loader, data)?;

        let forced_channel_ref = match loader.get_minor_version() >= 20 {
            true => loader.read_pointer(data)?,
            false => None,
        };

        Ok(Self { inner, forced_channel_ref })
    }
}

impl Deref for MovingPartBase {
    type Target = PartGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MovingPartBase {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
