use super::prelude::*;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub(crate) enum CullMode {
    None,
    #[default]
    Clockwise,
    CounterClockwise,
    Unchanged,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CullFaceAttrib {
    pub mode: CullMode,
    pub reverse: bool,
}

impl CullFaceAttrib {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mode = CullMode::from(data.read_u8()?);
        let reverse = data.read_bool()?;
        Ok(Self { mode, reverse })
    }
}
