use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct CollisionNode {
    /// CollisionNode is a superclass of a PandaNode, so we include its data here
    node: PandaNode,
    /// References to all associated CollisionSolid data
    solid_refs: Vec<u32>,
    collide_mask: u32,
}

impl CollisionNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        let mut num_solids = data.read_u16()? as u32;
        if num_solids == 0xFFFF {
            num_solids = data.read_u32()?;
        }
        let mut solid_refs = Vec::with_capacity(num_solids as usize);
        for _ in 0..num_solids {
            solid_refs.push(loader.read_pointer(data)?.unwrap());
        }

        let collide_mask = data.read_u32()?;

        Ok(Self { node, solid_refs, collide_mask })
    }
}
