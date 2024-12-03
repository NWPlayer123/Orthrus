use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct Character {
    pub inner: PartBundleNode,
    temp_part_refs: Vec<u32>,
}

impl Node for Character {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PartBundleNode::create(loader, data)?;

        // For compatibility only, no longer used and handled by PartBundleNode
        let temp_num_parts = data.read_u16()?;
        let mut temp_part_refs = Vec::with_capacity(temp_num_parts as usize);
        for _ in 0..temp_num_parts {
            temp_part_refs.push(loader.read_pointer(data)?.unwrap());
        }

        Ok(Self { inner, temp_part_refs })
    }
}

impl Deref for Character {
    type Target = PartBundleNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Character {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
