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

impl GraphDisplay for CollisionSphere {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{CollisionSphere|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(label, "|center: {}", self.center)?;
        write!(label, "|radius: {}", self.radius)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
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
