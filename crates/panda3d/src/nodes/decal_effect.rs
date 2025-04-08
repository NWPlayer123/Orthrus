use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct DecalEffect;

impl Node for DecalEffect {
    #[inline]
    fn create(_loader: &mut BinaryAsset, _data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self {})
    }
}

impl GraphDisplay for DecalEffect {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // This has no fields, let's just use one write
        write!(label, "{{DecalEffect}}")?;
        Ok(())
    }
}
