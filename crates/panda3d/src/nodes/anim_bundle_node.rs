use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimBundleNode {
    pub inner: PandaNode,
    pub anim_bundle_ref: u32,
}

impl Node for AnimBundleNode {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PandaNode::create(loader, data)?;
        let anim_bundle_ref = loader.read_pointer(data)?.unwrap();
        Ok(Self { inner, anim_bundle_ref })
    }
}

impl GraphDisplay for AnimBundleNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{AnimBundleNode|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        connections.push(self.anim_bundle_ref);

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for AnimBundleNode {
    type Target = PandaNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AnimBundleNode {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
