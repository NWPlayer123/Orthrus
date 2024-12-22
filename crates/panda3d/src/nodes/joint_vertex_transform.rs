use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct JointVertexTransform {
    pub joint_ref: u32,
}

impl Node for JointVertexTransform {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { joint_ref: loader.read_pointer(data)?.unwrap() })
    }
}

impl GraphDisplay for JointVertexTransform {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // This doesn't have any actual data, so write a placeholder
        write!(label, "{{JointVertexTransform}}")?;
        connections.push(self.joint_ref);
        Ok(())
    }
}

impl Deref for JointVertexTransform {
    type Target = u32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.joint_ref
    }
}

impl DerefMut for JointVertexTransform {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.joint_ref
    }
}
