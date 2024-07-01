use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct Character {
    pub node: PartBundleNode,
    pub temp_part_refs: Vec<u32>,
}

impl Character {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PartBundleNode::create(loader, data)?;
        let temp_num_parts = data.read_u16()?;
        let mut temp_part_refs = Vec::with_capacity(temp_num_parts as usize);
        for _ in 0..temp_num_parts {
            //TODO: we should relegate this to PartBundleNode, this is compatibility
            temp_part_refs.push(loader.read_pointer(data)?.unwrap());
        }
        Ok(Self { node, temp_part_refs })
    }
}
