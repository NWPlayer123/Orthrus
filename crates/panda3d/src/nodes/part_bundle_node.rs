use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct PartBundleNode {
    pub inner: PandaNode,
    pub bundle_refs: Vec<u32>,
}

impl PartBundleNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PandaNode::create(loader, data)?;

        let num_bundles = match loader.get_minor_version() >= 5 {
            true => data.read_u16()?,
            false => 1,
        };
        let mut bundle_refs = Vec::with_capacity(num_bundles as usize);
        for _ in 0..num_bundles {
            bundle_refs.push(loader.read_pointer(data)?.unwrap());
        }

        Ok(Self { inner, bundle_refs })
    }
}

impl GraphDisplay for PartBundleNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{PartBundleNode|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        for reference in &self.bundle_refs {
            connections.push(*reference);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for PartBundleNode {
    type Target = PandaNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PartBundleNode {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
