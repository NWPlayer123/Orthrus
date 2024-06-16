use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum Mode {
    /// No transparency.
    #[default]
    None,
    /// Normal transparency, sorted back to front.
    Alpha,
    /// Assume textures already have premultiplied alpha.
    PremultipliedAlpha,
    /// Uses multisample buffer, alpha values modified to 1.0.
    Multisample,
    /// Uses multisample buffer, alpha values unmodified.
    MultisampleMask,
    /// Only writes pixels if their alpha is at or above 0.5.
    Binary,
    /// Write opaque parts first, then sorted transparent parts.
    Dual,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TransparencyAttrib {
    mode: Mode,
}

impl TransparencyAttrib {
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mode = Mode::from(data.read_u8()?);
        Ok(Self { mode })
    }
}
