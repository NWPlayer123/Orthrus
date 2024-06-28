use super::prelude::*;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub(crate) enum DepthMode {
    Off,
    #[default]
    On,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct DepthWriteAttrib {
    mode: DepthMode,
}

impl DepthWriteAttrib {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { mode: DepthMode::from(data.read_u8()?) })
    }
}
