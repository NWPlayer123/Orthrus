use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum AutoTextureScale {
    None,
    Down,
    Up,
    Pad,
    #[default]
    Unspecified,
}
