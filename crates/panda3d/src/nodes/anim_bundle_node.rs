use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimBundleNode {
    node: PandaNode,
    anim_bundle_ref: u32,
}

impl AnimBundleNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;
        let anim_bundle_ref = loader.read_pointer(data)?.unwrap();
        Ok(Self { node, anim_bundle_ref })
    }
}
