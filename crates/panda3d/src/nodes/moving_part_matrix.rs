use core::ops::{Deref, DerefMut};

use super::prelude::*;

//TODO: This is technically a generic but I don't want to make it a generic right now
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct MovingPartMatrix {
    pub inner: MovingPartBase,
    pub value: Mat4,
    pub default_value: Mat4,
}

impl MovingPartMatrix {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = MovingPartBase::create(loader, data)?;

        let value = Mat4::read(data)?;
        let default_value = Mat4::read(data)?;

        Ok(Self { inner, value, default_value })
    }
}

impl Deref for MovingPartMatrix {
    type Target = MovingPartBase;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MovingPartMatrix {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
