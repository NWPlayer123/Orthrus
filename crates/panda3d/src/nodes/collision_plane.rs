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

impl GraphDisplay for CollisionPlane {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{CollisionPlane|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(label, "|plane: {}", self.plane)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
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
