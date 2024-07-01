use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct NodePath {
    path_refs: Vec<u32>,
}

impl NodePath {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mut path = Self::default();

        loop {
            match loader.read_pointer(data)? {
                Some(value) => path.path_refs.push(value),
                None => break,
            }
        }

        Ok(path)
    }
}
