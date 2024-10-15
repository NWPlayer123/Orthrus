use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct InternalName {
    pub name: String,
}

impl InternalName {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { name: data.read_string()? })
    }
}
