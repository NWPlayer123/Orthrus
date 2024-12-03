use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum Mode {
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
pub(crate) enum CombineMode {
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
pub(crate) enum CombineSource {
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
pub(crate) enum CombineOperand {
    #[default]
    Undefined,
    SourceColor,
    OneMinusSourceColor,
    SourceAlpha,
    OneMinusSourceAlpha,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CombineConfig {
    pub mode: CombineMode,
    pub num_operands: u8,
    pub sources: [CombineSource; 3],
    pub operands: [CombineOperand; 3],
}

//TODO: make the flag check functions const, need const PartialEq which is only in nightly rn
impl CombineConfig {
    #[inline]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
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

    #[inline]
    fn involves_color_scale(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::ConstantColorScale)
    }

    #[inline]
    fn uses_color(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::Constant)
    }

    #[inline]
    fn uses_primary_color(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::PrimaryColor)
    }

    #[inline]
    fn uses_last_saved_result(&self) -> bool {
        self.sources.iter().any(|&source| source == CombineSource::LastSavedResult)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct TextureStage {
    pub name: String,
    pub sort: i32,
    pub priority: i32,

    /// Reference to the InternalName for this node, used for UV calculations
    pub texcoord_name_ref: Option<u32>,

    pub mode: Mode,
    pub color: Vec4,
    pub rgb_scale: u8,
    pub alpha_scale: u8,
    /// Allows for caching and being reused as the TextureStage for multiple stages
    pub saved_result: bool,
    pub tex_view_offset: i32,

    pub combine_rgb: CombineConfig,
    pub combine_alpha: CombineConfig,
    pub involves_color_scale: bool,
    pub uses_color: bool,
    pub uses_primary_color: bool,
    pub uses_last_saved_result: bool,
}

impl TextureStage {
    #[inline]
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

impl Node for TextureStage {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        //Check if we're just using the default, otherwise load in all the config data
        if data.read_bool()? {
            return Ok(Self::default());
        }

        let name = data.read_string()?;
        let sort = data.read_i32()?;
        let priority = data.read_i32()?;

        let texcoord_name_ref = loader.read_pointer(data)?;

        let mode = Mode::from(data.read_u8()?);
        //TODO: define custom LColor type?
        let color = Vec4::read(data)?;
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
            texcoord_name_ref,
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
}

impl Default for TextureStage {
    #[inline]
    fn default() -> Self {
        Self {
            name: "default".to_owned(),
            sort: 0,
            priority: 0,
            texcoord_name_ref: None, //TODO: emit InternalName? needs a finalize()
            mode: Mode::default(),
            color: Vec4::W,
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
