use super::auto_texture_scale::AutoTextureScale;
use super::geom_enums::UsageHint;
use super::prelude::*;
use super::sampler_state::SamplerState;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub enum TextureType {
    Texture1D,
    #[default]
    Texture2D,
    Texture3D,
    Texture2DArray,
    CubeMap,
    BufferTexture,
    CubeMapArray,
    Texture1DArray,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum CompressionMode {
    // Generic compression modes. You should usually choose one of these.
    #[default]
    Default,
    Off,
    On,

    // Specific compression modes. You should only use these when you really want to use a
    // specific compression algorithm.
    /// 3DFX Texture Compression 1: older compression format
    FXT1,
    /// DirectX Texture Compression BC1: RGB with optional binary alpha
    DXT1,
    /// DirectX Texture Compression BC2: Like DXT3, but with premultipied alpha
    DXT2,
    /// DirectX Texture Compression BC2: RGB with uncompressed 4-bit alpha
    DXT3,
    /// DirectX Texture Compression BC3: Like DXT5, but with premultiplied alpha
    DXT4,
    /// DirectX Texture Compression BC3: RGB with separately compressed 8-bit alpha
    DXT5,
    /// PowerVR Texture Compression 1: 2 bit-per-pixel
    PVR1_2BPP,
    /// PowerVR Texture Compression 1: 4 bit-per-pixel
    PVR1_4BPP,
    /// Red/Green Texture Compression BC4/BC5: 1-2 channels, individually compressed
    RGTC,
    /// Ericsson Texture Compression 1: only supports RGB
    ETC1,
    /// Ericsson Texture Compression 2: supports full color, along with alpha
    ETC2,
    /// Ericsson Texture Compression EAC: only 1-2 channels
    EAC,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum QualityLevel {
    #[default]
    Default,
    Fastest,
    Normal,
    Best,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum Format {
    DepthStencil = 1,
    ColorIndex,
    Red,
    Green,
    Blue,
    Alpha,
    #[default]
    RGB, // Preferred RGB format

    RGB5,   // 5 bits for R, G, B
    RGB8,   // 8 bits for R, G, B
    RGB12,  // 12 bits for R, G, B
    RGB332, // 3 bits for R & G, 2 bits for B

    RGBA, // Preferred RGBA format

    RGBM,   // RGB with a 1-bit alpha mask
    RGBA4,  // 4 bits for R, G, B, A
    RGBA5,  // 5 bits for R, G, B, 1 bit for A
    RGBA8,  // 8 bits for R, G, B, A
    RGBA12, // 12 bits for R, G, B, A

    Luminance,
    LuminanceAlpha,     // 8 bits luminance, 8 bits alpha
    LuminanceAlphaMask, // 8 bits luminance, 1 bit alpha

    RGBA16, // 16 bits per channel
    RGBA32, // 32 bits per channel

    DepthComponent,
    DepthComponent16,
    DepthComponent24,
    DepthComponent32,

    R16,
    RG16,
    RGB16,

    SRGB,
    SRGBAlpha,
    SLuminance,
    SLuminanceAlpha,

    R32I, // 32-bit integer
    R32,
    RG32,
    RGB32,

    R8I,    // 8-bit integer per Red
    RG8I,   // 8-bit integer per RG
    RGB8I,  // 8-bit integer per RGB
    RGBA8I, // 8-bit integer per RGBA

    R11G11B10, // Unsigned floating-point
    RGB9E5,
    RGB10A2,

    RG,

    R16I,
    RG16I,
    RGB16I,
    RGBA16I,

    RG32I,
    RGB32I,
    RGBA32I,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum ComponentType {
    #[default]
    UnsignedByte,
    UnsignedShort,
    Float,
    UnsignedInt24, //packed
    Int,
    Byte,
    Short,
    HalfFloat, //cursed
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct Texture {
    pub name: String,
    pub filename: String,
    pub alpha_filename: String,

    pub color_num_channels: u8,
    pub alpha_num_channels: u8,
    pub has_rawdata: bool,
    pub texture_type: TextureType,
    pub body: TextureBody,
    pub data: Option<TextureData>,
    pub has_read_mipmaps: bool,
}

#[derive(Debug, Default)]
pub(crate) struct TextureBody {
    pub default_sampler: SamplerState,
    format: Format,
    compression: CompressionMode,
    usage_hint: UsageHint,
    quality_level: QualityLevel,
    auto_texture_scale: AutoTextureScale,
    num_components: u8,
    orig_file_x_size: u32,
    orig_file_y_size: u32,
    simple_x_size: u32,
    simple_y_size: u32,
    /// Timestamp of when the image was last modified
    simple_image_date_generated: i32,
    image: Vec<u8>,
    clear_color: Option<Vec4>,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TextureData {
    size: UVec3,
    pad_size: UVec3,
    num_views: u32,
    component_type: ComponentType,
    component_width: u8,
    ram_image_compression: CompressionMode,
    ram_image_count: u8,
    /// Page Size + Image Data
    ram_images: Vec<(u32, Vec<u8>)>,
}

//TODO: Textures can be cached via a TexturePool
impl Texture {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name = data.read_string()?;
        let filename = data.read_string()?;
        let alpha_filename = data.read_string()?;

        let color_num_channels = data.read_u8()?;
        let alpha_num_channels = data.read_u8()?;
        let has_rawdata = data.read_bool()?;
        let mut texture_type = TextureType::from(data.read_u8()?);
        if loader.get_minor_version() < 25 {
            // As of Panda3D 1.8.0/BAM v6.25, Texture2DArray was added as a TextureType, so we need to account
            // for the shift
            if texture_type == TextureType::Texture2DArray {
                texture_type = TextureType::CubeMap;
            }
        }

        let has_read_mipmaps = match loader.get_minor_version() >= 32 {
            true => data.read_bool()?,
            false => false,
        };

        let mut texture = Self {
            name,
            filename,
            alpha_filename,
            color_num_channels,
            alpha_num_channels,
            has_rawdata,
            texture_type,
            has_read_mipmaps,
            ..Default::default()
        };

        texture.body = texture.fillin_body(loader, data)?;

        if has_rawdata == true {
            //The texture data is included in this BAM file so we need to actually load it
            texture.data = Some(texture.fillin_data(loader, data)?);
        } else {
            //Otherwise, the raw image data isn't stored, so we need to load the relevant image from VFS
            //texture.data = std::fs::read(), remove the Option<>
        }

        Ok(texture)
    }

    #[inline]
    fn fillin_body(&self, loader: &mut BinaryAsset, data: &mut Datagram) -> Result<TextureBody, bam::Error> {
        let mut body = TextureBody::default();

        body.default_sampler = SamplerState::create(loader, data)?;

        if loader.get_minor_version() >= 1 {
            body.compression = CompressionMode::from(data.read_u8()?);
        }

        if loader.get_minor_version() >= 16 {
            body.quality_level = QualityLevel::from(data.read_u8()?);
        }

        body.format = Format::from(data.read_u8()?);
        body.num_components = data.read_u8()?;

        if self.texture_type == TextureType::BufferTexture {
            body.usage_hint = UsageHint::from(data.read_u8()?);
        }

        //properties_modified++;

        body.auto_texture_scale = match loader.get_minor_version() >= 28 {
            true => AutoTextureScale::from(data.read_u8()?),
            false => AutoTextureScale::Unspecified,
        };

        let mut has_simple_ram_image = false;
        if loader.get_minor_version() >= 18 {
            body.orig_file_x_size = data.read_u32()?;
            body.orig_file_y_size = data.read_u32()?;
            has_simple_ram_image = data.read_bool()?;
        }

        if has_simple_ram_image {
            body.simple_x_size = data.read_u32()?;
            body.simple_y_size = data.read_u32()?;
            body.simple_image_date_generated = data.read_i32()?;
            let size = data.read_u32()?;
            body.image = vec![0u8; size as usize];
            data.read_length(&mut body.image)?;
        }

        if loader.get_minor_version() >= 45 && data.read_bool()? {
            body.clear_color = Some(Vec4::read(data)?);
        }
        Ok(body)
    }

    #[inline]
    fn fillin_data(&self, loader: &mut BinaryAsset, data: &mut Datagram) -> Result<TextureData, bam::Error> {
        let size = UVec3::read(data)?;

        let pad_size = match loader.get_minor_version() >= 30 {
            true => UVec3::read(data)?,
            false => UVec3::ZERO,
        };

        let num_views = match loader.get_minor_version() >= 26 {
            true => data.read_u32()?,
            false => 1,
        };
        let component_type = ComponentType::from(data.read_u8()?);
        let component_width = data.read_u8()?;
        let ram_image_compression = match loader.get_minor_version() >= 1 {
            true => CompressionMode::from(data.read_u8()?),
            false => CompressionMode::Off,
        };

        let ram_image_count = match loader.get_minor_version() >= 3 {
            true => data.read_u8()?,
            false => 1,
        };

        let mut ram_images = Vec::with_capacity(ram_image_count as usize);
        for _ in 0..ram_image_count {
            let page_size = match loader.get_minor_version() >= 1 {
                true => data.read_u32()?,
                false => todo!("I have no BAM files this old - contact me"),
            };
            let size = data.read_u32()?;
            let mut allocation = vec![0; size as usize];
            allocation.copy_from_slice(&data.read_slice(size as usize)?);
            ram_images.push((page_size, allocation));
        }

        Ok(TextureData {
            size,
            pad_size,
            num_views,
            component_type,
            component_width,
            ram_image_compression,
            ram_image_count,
            ram_images,
        })
    }

    #[allow(dead_code)]
    pub fn has_simple_ram_image(&self) -> bool {
        !self.body.image.is_empty()
    }
}
