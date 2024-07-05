use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct JointVertexTransform {
    pub joint_ref: u32,
}

impl JointVertexTransform {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { joint_ref: loader.read_pointer(data)?.unwrap() })
    }
}
