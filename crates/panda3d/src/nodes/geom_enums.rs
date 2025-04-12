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
    /// The data will be modified at runtime and re-rendered. Used for data modified at runtime like animated
    /// or soft-skinned vertices.
    Dynamic,
    /// The data will be created once, and used to render many times without being modified. This is the most
    /// common, since usually vertex data isn't directly animated.
    Static,
    /// The usage is unspecified, intended as a "don't care" option for abstract objects. It should not be
    /// used on any rendered geometry.
    #[default]
    Unspecified,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum NumericType {
    #[default]
    U8,
    U16,
    U32,
    /// DirectX ABGR
    PackedDCBA,
    /// DirectX ARGB
    PackedDABC,
    F32,
    F64,
    /// Single/Double-Precision Float
    StdFloat,
    I8,
    I16,
    I32,
    /// Three 10/11-bit float components packed in a u32
    PackedUFloat,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum Contents {
    #[default]
    Other,
    Point,
    ClipPoint,
    Vector,
    TexCoord,
    Color,
    Index,
    MorphDelta,
    Matrix,
    Normal,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum ShadeModel {
    #[default]
    Uniform,
    Smooth,
    FlatFirstVertex,
    FlatLastVertex,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum PrimitiveType {
    #[default]
    None,
    Polygons,
    Lines,
    Points,
    Patches,
}

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct GeomRendering: u32 {
        const IndexedPoint         = 1 << 0;
        const IndexedOther         = 1 << 16;
        const IndexedBits          = Self::IndexedPoint.bits() | Self::IndexedOther.bits();
        const Point                = 1 << 1;
        const PointUniformSize     = 1 << 2;
        const PerPointSize         = 1 << 3;
        const PointPerspective     = 1 << 4;
        const PointAspectRatio     = 1 << 5;
        const PointScale           = 1 << 6;
        const PointRotate          = 1 << 7;
        const PointSprite          = 1 << 8;
        const PointSpriteTexMatrix = 1 << 9;
        const PointBits            = Self::Point.bits() | Self::PointUniformSize.bits() | Self::PerPointSize.bits()
                                    | Self::PointPerspective.bits() | Self::PointAspectRatio.bits()
                                    | Self::PointScale.bits() | Self::PointRotate.bits() | Self::PointSprite.bits()
                                    | Self::PointSpriteTexMatrix.bits();
        const TriangleStrip        = 1 << 10;
        const TriangleFan          = 1 << 11;
        const LineStrip            = 1 << 12;
        const CompositeBits        = Self::TriangleStrip.bits() | Self::TriangleFan.bits() | Self::LineStrip.bits();
        const StripCutIndex        = 1 << 17;
        const FlatFirstVertex      = 1 << 13;
        const FlatLastVertex       = 1 << 14;
        const ShadeModelBits       = Self::FlatFirstVertex.bits() | Self::FlatLastVertex.bits();
        const RenderModeWireframe  = 1 << 18;
        const RenderModePoint      = 1 << 19;
        const Adjacency            = 1 << 20;
    }
}
