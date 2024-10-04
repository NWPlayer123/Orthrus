use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct PartGroup {
    pub name: String,
    pub child_refs: Vec<u32>,
}

impl PartGroup {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;
        if loader.get_minor_version() == 11 {
            panic!("I don't have a BAM file this old - contact me");
        }
        let num_children = data.read_u16()?;
        let mut child_refs = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            child_refs.push(loader.read_pointer(data)?.unwrap());
        }
        Ok(Self { name, child_refs })
    }
}
