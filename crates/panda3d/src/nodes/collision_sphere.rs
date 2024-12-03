use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CollisionSphere {
    pub inner: CollisionSolid,
    pub center: Vec3,
    pub radius: f32,
}

impl Node for CollisionSphere {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = CollisionSolid::create(loader, data)?;
        let center = Vec3::read(data)?;
        let radius = data.read_float()?;
        Ok(Self { inner, center, radius })
    }
}

impl Deref for CollisionSphere {
    type Target = CollisionSolid;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CollisionSphere {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
