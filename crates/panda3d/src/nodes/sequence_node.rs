use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct SequenceNode {
    // SelectiveChildNode just inherits from a PandaNode.
    pub inner: PandaNode,
    pub interface: AnimInterface,
}

impl Node for SequenceNode {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PandaNode::create(loader, data)?;
        let interface = AnimInterface::create(loader, data)?;

        Ok(Self { inner, interface })
    }
}

impl GraphDisplay for SequenceNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{SequenceNode|")?;
        }

        self.inner.write_data(label, connections, false)?;
        write!(label, "|")?;
        self.interface.write_data(label, connections, false)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for SequenceNode {
    type Target = PandaNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SequenceNode {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
