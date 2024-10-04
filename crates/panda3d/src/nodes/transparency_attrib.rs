use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum TransparencyMode {
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
pub(crate) struct TransparencyAttrib {
    pub mode: TransparencyMode,
}

impl TransparencyAttrib {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { mode: TransparencyMode::from(data.read_u8()?) })
    }
}
