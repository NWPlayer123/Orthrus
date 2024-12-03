use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct DecalEffect;

impl Node for DecalEffect {
    #[inline]
    fn create(_loader: &mut BinaryAsset, _data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        Ok(Self {})
    }
}
