use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimGroup {
    name: String,
    root_ref: u32,
    child_refs: Vec<u32>,
}

impl AnimGroup {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;
        let root_ref = loader.read_pointer(data)?.unwrap();
        let num_children = data.read_u16()?;
        let mut child_refs = Vec::with_capacity(num_children as usize);
        for _ in 0..num_children {
            let child_ref = loader.read_pointer(data)?.unwrap();
            child_refs.push(child_ref);
        }
        Ok(Self { name, root_ref, child_refs })
    }
}
