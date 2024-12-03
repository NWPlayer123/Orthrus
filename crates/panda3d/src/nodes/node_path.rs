use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct NodePath {
    pub path_refs: Vec<u32>,
}

impl NodePath {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mut path_refs = Vec::new();

        while let Some(value) = loader.read_pointer(data)? {
            path_refs.push(value);
        }

        Ok(Self { path_refs })
    }
}
