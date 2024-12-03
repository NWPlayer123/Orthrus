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
