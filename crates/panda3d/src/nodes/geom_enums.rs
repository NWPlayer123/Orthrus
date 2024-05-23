use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum AnimationType {
    #[default]
    /// No vertex animation performed.
    None,
    /// Animations are processed on the CPU through Panda3D.
    Panda,
    /// Animations are hardware-accelerated on the GPU.
    Hardware,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum UsageHint {
    // These are ordered from most dynamic to most static.
    /// Don't attempt to upload the data, always keep it on the client.
    Client,
    /// The data will be created once, used to render a few times, and then discarded.
    Stream,
    /// The data will be modified at runtime and re-rendered. Used for data modified at runtime
    /// like animated or soft-skinned vertices.
    Dynamic,
    /// The data will be created once, and used to render many times without being modified. This
    /// is the most common, since usually vertex data isn't directly animated.
    Static,
    /// The usage is unspecified, intended as a "don't care" option for abstract objects. It should
    /// not be used on any rendered geometry.
    #[default]
    Unspecified,
}
