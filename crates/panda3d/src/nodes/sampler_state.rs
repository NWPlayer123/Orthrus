use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum WrapMode {
    /// Clamp coordinate to [0, 1]
    Clamp,
    #[default]
    Repeat,
    Mirror,
    /// Mirror once, and then clamp
    MirrorOnce,
    /// Coordinates outside [0, 1] use an explicit border color
    BorderColor,
    Invalid,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum FilterType {
    // Both min filter and mag filter
    /// Point sample of each pixel
    Nearest,
    /// Bilinear filtering of four neighboring pixels
    Linear,

    // Only min filter
    /// Point sample the pixel from the nearest mipmap level
    NearestMipmapNearest,
    /// Bilinear filter the pixel from the nearest mipmap level
    LinearMipmapNearest,
    /// Point sample the pixel from two mipmap levels, and linearly blend
    NearestMipmapLinear,
    /// Trilinear filtering; Bilinear filter the pixel from two mipmap levels, and linearly blend
    LinearMipmapLinear,
    /// This uses the OpenGL ARB_shadow extension
    Shadow,

    /// Default depends on the format, usually linear.
    #[default]
    Default,
    Invalid,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct SamplerState {
    pub wrap_u: WrapMode,
    pub wrap_v: WrapMode,
    pub wrap_w: WrapMode,

    pub min_filter: FilterType,
    pub mag_filter: FilterType,

    pub aniso_degree: i16,
    //TODO: custom LColor variable?
    pub border_color: Vec4,

    pub min_lod: f32,
    pub max_lod: f32,
    pub lod_bias: f32,
}

impl SamplerState {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let wrap_u = WrapMode::from(data.read_u8()?);
        let wrap_v = WrapMode::from(data.read_u8()?);
        let wrap_w = WrapMode::from(data.read_u8()?);

        let min_filter = FilterType::from(data.read_u8()?);
        let mag_filter = FilterType::from(data.read_u8()?);

        let aniso_degree = data.read_i16()?;

        let border_color = Vec4::read(data)?;

        let (min_lod, max_lod, lod_bias) = match loader.get_minor_version() >= 36 {
            true => (data.read_float()?, data.read_float()?, data.read_float()?),
            false => (-1000.0, 1000.0, 0.0),
        };

        Ok(Self {
            wrap_u,
            wrap_v,
            wrap_w,
            min_filter,
            mag_filter,
            aniso_degree,
            border_color,
            min_lod,
            max_lod,
            lod_bias,
        })
    }
}

impl Default for SamplerState {
    #[inline]
    fn default() -> Self {
        Self {
            wrap_u: WrapMode::default(),
            wrap_v: WrapMode::default(),
            wrap_w: WrapMode::default(),

            min_filter: FilterType::default(),
            mag_filter: FilterType::default(),

            aniso_degree: 0,
            border_color: Vec4::W,

            min_lod: -1000.0,
            max_lod: 1000.0,
            lod_bias: 0.0,
        }
    }
}
