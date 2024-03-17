use crate::nodes::dispatch::{DatagramRead, DatagramWrite};
use crate::{
    bam::{self, BinaryAsset},
    common::Datagram,
};
use orthrus_core::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
enum Mode {
    #[default]
    //fixed-function pipeline
    Modulate,
    Decal,
    Blend,
    Replace,
    Add,
    Combine,
    BlendColorScale,

    /// Equivalent to Modulate when used in fixed-function pipeline.
    ModulateGlow,
    /// Equivalent to Modulate when used in fixed-function pipeline.
    ModulateGloss,

    //shader-based pipeline
    Normal,
    NormalHeight,
    /// Rarely used, ModulateGlow is more efficient.
    Glow,
    /// Rarely used, ModulateGloss is more efficient.
    Gloss,
    /// Rarely used, NormalHeight is more efficient.
    Height,
    Selector,
    NormalGloss,
    Emission,
}

impl TryFrom<u8> for Mode {
    type Error = bam::Error;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => Mode::Modulate,
            1 => Mode::Decal,
            2 => Mode::Blend,
            3 => Mode::Replace,
            4 => Mode::Add,
            5 => Mode::Combine,
            6 => Mode::BlendColorScale,
            7 => Mode::ModulateGlow,
            8 => Mode::ModulateGloss,
            9 => Mode::Normal,
            10 => Mode::NormalHeight,
            11 => Mode::Glow,
            12 => Mode::Gloss,
            13 => Mode::Height,
            14 => Mode::Selector,
            15 => Mode::NormalGloss,
            16 => Mode::Emission,
            _ => return Err(bam::Error::InvalidEnum),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
enum CombineMode {
    #[default]
    Undefined,
    Replace,
    Modulate,
    Add,
    AddSigned,
    Interpolate,
    Subtract,
    DotProduct3RGB,
    DotProduct3RGBA,
}

impl TryFrom<u8> for CombineMode {
    type Error = bam::Error;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => CombineMode::Undefined,
            1 => CombineMode::Replace,
            2 => CombineMode::Modulate,
            3 => CombineMode::Add,
            4 => CombineMode::AddSigned,
            5 => CombineMode::Interpolate,
            6 => CombineMode::Subtract,
            7 => CombineMode::DotProduct3RGB,
            8 => CombineMode::DotProduct3RGBA,
            _ => return Err(bam::Error::InvalidEnum),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
enum CombineSource {
    #[default]
    Undefined,
    Texture,
    Constant,
    PrimaryColor,
    Previous,
    ConstantColorScale,
    LastSavedResult,
}

impl TryFrom<u8> for CombineSource {
    type Error = bam::Error;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => CombineSource::Undefined,
            1 => CombineSource::Texture,
            2 => CombineSource::Constant,
            3 => CombineSource::PrimaryColor,
            4 => CombineSource::Previous,
            5 => CombineSource::ConstantColorScale,
            6 => CombineSource::LastSavedResult,
            _ => return Err(bam::Error::InvalidEnum),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
enum CombineOperand {
    #[default]
    Undefined,
    SourceColor,
    OneMinusSourceColor,
    SourceAlpha,
    OneMinusSourceAlpha,
}

impl TryFrom<u8> for CombineOperand {
    type Error = bam::Error;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => CombineOperand::Undefined,
            1 => CombineOperand::SourceColor,
            2 => CombineOperand::OneMinusSourceColor,
            3 => CombineOperand::SourceAlpha,
            4 => CombineOperand::OneMinusSourceAlpha,
            _ => return Err(bam::Error::InvalidEnum),
        })
    }
}

#[derive(Default, Debug)]
#[allow(dead_code)]
struct CombineConfig {
    mode: CombineMode,
    num_operands: u8,
    sources: [CombineSource; 3],
    operands: [CombineOperand; 3],
}

//TODO: make the flag check functions const, need const PartialEq which is only in nightly rn
impl CombineConfig {
    fn new(data: &mut Datagram) -> Result<Self, crate::bam::Error> {
        let mode: CombineMode = data.read_u8()?.try_into()?;
        let num_operands = data.read_u8()?;
        let source0: CombineSource = data.read_u8()?.try_into()?;
        let operand0: CombineOperand = data.read_u8()?.try_into()?;
        let source1: CombineSource = data.read_u8()?.try_into()?;
        let operand1: CombineOperand = data.read_u8()?.try_into()?;
        let source2: CombineSource = data.read_u8()?.try_into()?;
        let operand2: CombineOperand = data.read_u8()?.try_into()?;

        Ok(Self {
            mode,
            num_operands,
            sources: [source0, source1, source2],
            operands: [operand0, operand1, operand2],
        })
    }

    fn involves_color_scale(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::ConstantColorScale)
    }

    fn uses_color(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::Constant)
    }

    fn uses_primary_color(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::PrimaryColor)
    }

    fn uses_last_saved_result(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::LastSavedResult)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct TextureStage {
    name: String,
    sort: i32,
    priority: i32,
    mode: Mode,
    //TODO: LVecBase4
    color: [f32; 4],
    rgb_scale: u8,
    alpha_scale: u8,
    saved_result: bool,
    tex_view_offset: i32,

    combine_rgb: CombineConfig,
    combine_alpha: CombineConfig,
    involves_color_scale: bool,
    uses_color: bool,
    uses_primary_color: bool,
    uses_last_saved_result: bool,
}

impl TextureStage {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, crate::bam::Error> {
        //Check if we're just using the default, otherwise load in all the config data
        if data.read_bool()? {
            return Ok(Self::default());
        }

        let name = data.read_string()?;
        let sort = data.read_i32()?;
        let priority = data.read_i32()?;

        loader.read_pointer(data)?;

        let mode: Mode = data.read_u8()?.try_into()?;
        //LColor -> LVecBase4
        let color = [
            data.read_f32()?,
            data.read_f32()?,
            data.read_f32()?,
            data.read_f32()?,
        ];
        let rgb_scale = data.read_u8()?;
        let alpha_scale = data.read_u8()?;
        let saved_result = data.read_bool()?;
        let tex_view_offset = match loader.get_minor_version() >= 26 {
            true => data.read_i32()?,
            false => 0,
        };

        let combine_rgb = CombineConfig::new(data)?;
        let combine_alpha = CombineConfig::new(data)?;

        let involves_color_scale = mode == Mode::BlendColorScale
            || (mode == Mode::Combine
                && (combine_rgb.involves_color_scale() || combine_alpha.involves_color_scale()));

        let uses_color = mode == Mode::Blend
            || mode == Mode::BlendColorScale
            || (mode == Mode::Combine && (combine_rgb.uses_color() || combine_alpha.uses_color()));

        let uses_primary_color = mode == Mode::Combine
            && (combine_rgb.uses_primary_color() || combine_alpha.uses_primary_color());

        let uses_last_saved_result = mode == Mode::Combine
            && (combine_rgb.uses_last_saved_result() || combine_alpha.uses_last_saved_result());

        Ok(Self {
            name,
            sort,
            priority,
            mode,
            color,
            rgb_scale,
            alpha_scale,
            saved_result,
            tex_view_offset,
            combine_rgb,
            combine_alpha,
            involves_color_scale,
            uses_color,
            uses_primary_color,
            uses_last_saved_result,
        })
    }
}

impl DatagramRead for TextureStage {
    fn finalize(&self) -> Result<(), crate::bam::Error> {
        Ok(())
    }
}

impl DatagramWrite for TextureStage {
    fn write(&self) -> Result<Datagram, crate::bam::Error> {
        Err(bam::Error::EndOfFile)
    }
}

impl Default for TextureStage {
    fn default() -> Self {
        Self {
            name: "default".to_owned(),
            sort: 0,
            priority: 0,
            mode: Mode::Modulate,
            color: [0.0, 0.0, 0.0, 1.0],
            rgb_scale: 1,
            alpha_scale: 1,
            saved_result: false,
            tex_view_offset: 0,
            combine_rgb: CombineConfig {
                mode: CombineMode::Undefined,
                num_operands: 0,
                sources: [
                    CombineSource::Undefined,
                    CombineSource::Undefined,
                    CombineSource::Undefined,
                ],
                operands: [
                    CombineOperand::Undefined,
                    CombineOperand::Undefined,
                    CombineOperand::Undefined,
                ],
            },
            combine_alpha: CombineConfig {
                mode: CombineMode::Undefined,
                num_operands: 0,
                sources: [
                    CombineSource::Undefined,
                    CombineSource::Undefined,
                    CombineSource::Undefined,
                ],
                operands: [
                    CombineOperand::Undefined,
                    CombineOperand::Undefined,
                    CombineOperand::Undefined,
                ],
            },
            involves_color_scale: false,
            uses_color: false,
            uses_primary_color: false,
            uses_last_saved_result: false,
        }
    }
}
