use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct InternalName {
    string: String,
}

impl InternalName {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { string: data.read_string()? })
    }
}
