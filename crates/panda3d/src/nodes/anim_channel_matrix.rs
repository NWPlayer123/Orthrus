use core::ops::{Deref, DerefMut};

use super::prelude::*;

// This is technically a generic but I don't feel like making one
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimChannelMatrix {
    pub inner: AnimGroup,
    pub last_frame: u16,
}

impl AnimChannelMatrix {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = AnimGroup::create(loader, data)?;
        let last_frame = data.read_u16()?;
        Ok(Self { inner, last_frame })
    }
}

impl Deref for AnimChannelMatrix {
    type Target = AnimGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AnimChannelMatrix {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
