use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum BoundsType {
    #[default]
    Default,
    Best,
    Sphere,
    Box,
    Fastest,
}
