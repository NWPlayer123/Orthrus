use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CollisionPlane {
    pub inner: CollisionSolid,
    pub plane: Vec4,
}

impl CollisionPlane {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = CollisionSolid::create(loader, data)?;
        let plane = Vec4::read(data)?;
        Ok(Self { inner, plane })
    }
}

impl Deref for CollisionPlane {
    type Target = CollisionSolid;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CollisionPlane {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
