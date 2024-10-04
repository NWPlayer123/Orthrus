use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct DecalEffect {}

impl DecalEffect {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, _data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self {})
    }
}
