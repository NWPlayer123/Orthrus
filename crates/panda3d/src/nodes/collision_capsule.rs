use core::ops::{Deref, DerefMut};

use bevy_transform::prelude::*;

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CollisionCapsule {
    pub inner: CollisionSolid,
    pub a: Vec3,
    pub b: Vec3,
    pub radius: f32,
    pub length: f32,
    pub transform: Transform,
}

impl CollisionCapsule {
    #[inline]
    fn recalc_internals(&mut self) {
        let direction = self.b - self.a;
        self.length = direction.length();

        self.transform.translation = self.a;
        self.transform.look_to(direction, Vec3::Y);

        // TODO: helper functions if we actually need transform.compute_matrix()/.inverse()?
    }
}

impl Node for CollisionCapsule {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = CollisionSolid::create(loader, data)?;

        let a = Vec3::read(data)?;
        let b = Vec3::read(data)?;
        let radius = data.read_float()?;

        let mut capsule = Self { inner, a, b, radius, ..Default::default() };

        capsule.recalc_internals();

        Ok(capsule)
    }
}

impl GraphDisplay for CollisionCapsule {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{CollisionCapsule|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(label, "|a: {}", self.a)?;
        write!(label, "|b: {}", self.b)?;
        write!(label, "|radius: {}", self.radius)?;
        write!(label, "|length: {}", self.length)?;
        write!(label, "|transform: {:?}", self.transform)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for CollisionCapsule {
    type Target = CollisionSolid;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CollisionCapsule {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
