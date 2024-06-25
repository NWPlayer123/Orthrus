use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum Mode {
    //fixed-function pipeline
    #[default]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum CombineOperand {
    #[default]
    Undefined,
    SourceColor,
    OneMinusSourceColor,
    SourceAlpha,
    OneMinusSourceAlpha,
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
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mode = CombineMode::from(data.read_u8()?);
        let num_operands = data.read_u8()?;
        let source0 = CombineSource::from(data.read_u8()?);
        let operand0 = CombineOperand::from(data.read_u8()?);
        let source1 = CombineSource::from(data.read_u8()?);
        let operand1 = CombineOperand::from(data.read_u8()?);
        let source2 = CombineSource::from(data.read_u8()?);
        let operand2 = CombineOperand::from(data.read_u8()?);

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

    texcoord_name: Option<u32>,

    mode: Mode,
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
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        //Check if we're just using the default, otherwise load in all the config data
        if data.read_bool()? {
            return Ok(Self::default());
        }

        let name = data.read_string()?;
        let sort = data.read_i32()?;
        let priority = data.read_i32()?;

        let texcoord_name = loader.read_pointer(data)?;

        let mode = Mode::from(data.read_u8()?);
        //LColor -> LVecBase4
        let color = [
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
        ];
        let rgb_scale = data.read_u8()?;
        let alpha_scale = data.read_u8()?;
        let saved_result = data.read_bool()?;
        let tex_view_offset = match loader.get_minor_version() >= 26 {
            true => data.read_i32()?,
            false => 0,
        };

        let combine_rgb = CombineConfig::create(loader, data)?;
        let combine_alpha = CombineConfig::create(loader, data)?;

        let mut stage = Self {
            name,
            sort,
            priority,
            texcoord_name,
            mode,
            color,
            rgb_scale,
            alpha_scale,
            saved_result,
            tex_view_offset,
            combine_rgb,
            combine_alpha,
            ..Default::default()
        };

        stage.update_color_flags();

        Ok(stage)
    }

    fn update_color_flags(&mut self) {
        self.involves_color_scale = self.mode == Mode::BlendColorScale
            || (self.mode == Mode::Combine
                && (self.combine_rgb.involves_color_scale() || self.combine_alpha.involves_color_scale()));

        self.uses_color = self.mode == Mode::Blend
            || self.mode == Mode::BlendColorScale
            || (self.mode == Mode::Combine
                && (self.combine_rgb.uses_color() || self.combine_alpha.uses_color()));

        self.uses_primary_color = self.mode == Mode::Combine
            && (self.combine_rgb.uses_primary_color() || self.combine_alpha.uses_primary_color());

        self.uses_last_saved_result = self.mode == Mode::Combine
            && (self.combine_rgb.uses_last_saved_result() || self.combine_alpha.uses_last_saved_result());
    }
}

impl Default for TextureStage {
    fn default() -> Self {
        Self {
            name: "default".to_owned(),
            sort: 0,
            priority: 0,
            texcoord_name: None,
            mode: Mode::default(),
            color: [0.0, 0.0, 0.0, 1.0],
            rgb_scale: 1,
            alpha_scale: 1,
            saved_result: false,
            tex_view_offset: 0,
            combine_rgb: CombineConfig::default(),
            combine_alpha: CombineConfig::default(),
            involves_color_scale: false,
            uses_color: false,
            uses_primary_color: false,
            uses_last_saved_result: false,
        }
    }
}
