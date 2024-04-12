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
    //TODO: lots of good candidates for sub-types
    wrap_u: WrapMode,
    wrap_v: WrapMode,
    wrap_w: WrapMode,

    min_filter: FilterType,
    mag_filter: FilterType,

    aniso_degree: i16,
    border_color: [f64; 4],

    min_lod: f64,
    max_lod: f64,
    lod_bias: f64,
}

impl SamplerState {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mut state = Self::default();
        state.wrap_u = WrapMode::from(data.read_u8()?);
        state.wrap_v = WrapMode::from(data.read_u8()?);
        state.wrap_w = WrapMode::from(data.read_u8()?);

        state.min_filter = FilterType::from(data.read_u8()?);
        state.mag_filter = FilterType::from(data.read_u8()?);

        state.aniso_degree = data.read_i16()?;

        //LColor -> [f64; 4]
        state.border_color = [
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
        ];

        if loader.get_minor_version() >= 36 {
            state.min_lod = data.read_float()?;
            state.max_lod = data.read_float()?;
            state.lod_bias = data.read_float()?;
        }

        Ok(state)
    }
}

impl Default for SamplerState {
    fn default() -> Self {
        Self {
            wrap_u: WrapMode::default(),
            wrap_v: WrapMode::default(),
            wrap_w: WrapMode::default(),

            min_filter: FilterType::default(),
            mag_filter: FilterType::default(),

            aniso_degree: 0,
            border_color: [0.0, 0.0, 0.0, 1.0],

            min_lod: -1000.0,
            max_lod: 1000.0,
            lod_bias: 0.0,
        }
    }
}
