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
