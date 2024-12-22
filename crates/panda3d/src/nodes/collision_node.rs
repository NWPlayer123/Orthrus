use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CollisionNode {
    pub inner: PandaNode,
    /// References to all associated CollisionSolid data
    pub solid_refs: Vec<u32>,
    pub collide_mask: u32,
}

impl Node for CollisionNode {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PandaNode::create(loader, data)?;

        let num_solids = match data.read_u16()? {
            0xFFFF => data.read_u32()?,
            num_solids => num_solids.into(),
        };
        let mut solid_refs = Vec::with_capacity(num_solids as usize);
        for _ in 0..num_solids {
            solid_refs.push(loader.read_pointer(data)?.unwrap());
        }

        let collide_mask = data.read_u32()?;

        Ok(Self { inner, solid_refs, collide_mask })
    }
}

impl GraphDisplay for CollisionNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{CollisionNode|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        for reference in &self.solid_refs {
            connections.push(*reference);
        }
        write!(label, "|collide_mask: {:#010X}", self.collide_mask)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for CollisionNode {
    type Target = PandaNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CollisionNode {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
