use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct PartBundleNode {
    pub node: PandaNode,
    pub bundle_refs: Vec<u32>,
}

impl PartBundleNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        let num_bundles = match loader.get_minor_version() >= 5 {
            true => data.read_u16()?,
            false => 1,
        };
        let mut bundle_refs = Vec::with_capacity(num_bundles as usize);
        for _ in 0..num_bundles {
            bundle_refs.push(loader.read_pointer(data)?.unwrap());
        }

        Ok(Self { node, bundle_refs })
    }
}
