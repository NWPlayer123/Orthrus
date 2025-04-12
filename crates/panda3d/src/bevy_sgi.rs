use bevy_asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader};
use bevy_image::Image;
use bevy_render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use orthrus_core::prelude::*;
use snafu::prelude::*;

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum SgiError {
    /// Thrown if a [`std::io::Error`] happened when trying to read/write files.
    #[snafu(display("Filesystem Error {source}"))]
    FileError { source: std::io::Error },

    /// Thrown if a data error occurred while reading.
    #[snafu(display("Decoding Error {source}"))]
    DataError { source: DataError },

    /// Thrown if the header contains a magic number other than "\x01\xDA"
    #[snafu(display("Invalid Magic! Expected {expected:?}."))]
    InvalidMagic { expected: &'static [u8] },

    /// Thrown if the dimension value is not 1, 2, or 3.
    #[snafu(display("Invalid dimension value: {value}. Expected 1, 2, or 3"))]
    InvalidDimension { value: u16 },

    /// Thrown if bytes per pixel is not 1 or 2.
    #[snafu(display("Unsupported bytes per pixel: {value}. Expected 1 or 2"))]
    UnsupportedBytesPerPixel { value: u8 },

    /// Thrown if number of channels is not 1, 3, or 4.
    #[snafu(display("Unsupported number of channels: {value}. Expected 1, 3, or 4"))]
    UnsupportedChannels { value: u16 },

    /// Thrown if RLE compressed data is invalid or corrupt.
    #[snafu(display("Invalid RLE compressed data"))]
    InvalidRleData,
}

impl From<DataError> for SgiError {
    #[inline]
    fn from(source: DataError) -> Self {
        Self::DataError { source }
    }
}

impl From<std::io::Error> for SgiError {
    #[inline]
    fn from(source: std::io::Error) -> Self {
        Self::FileError { source }
    }
}

#[derive(Debug)]
struct SgiHeader {
    compression: u8,
    bytes_per_pixel: u8,
    dimension: u16,
    width: u16,
    height: u16,
    channels: u16,
    _min_value: u32,
    _max_value: u32,
    _image_name: [u8; 80],
    _colormap: u32,
}

impl SgiHeader {
    pub const MAGIC: &'static [u8] = &[0x01, 0xDA];

    fn read<T: ReadExt>(data: &mut T) -> Result<Self, SgiError> {
        let magic = data.read_exact::<2>()?;
        ensure!(magic == Self::MAGIC, InvalidMagicSnafu { expected: Self::MAGIC });

        let compression = data.read_u8()?;
        let bytes_per_pixel = data.read_u8()?;
        ensure!(
            bytes_per_pixel == 1 || bytes_per_pixel == 2,
            UnsupportedBytesPerPixelSnafu { value: bytes_per_pixel }
        );

        let dimension = data.read_u16()?;
        ensure!((1..=3).contains(&dimension), InvalidDimensionSnafu { value: dimension });

        let width = data.read_u16()?;
        let height = data.read_u16()?;
        let channels = data.read_u16()?;
        ensure!(
            channels == 1 || channels == 3 || channels == 4,
            UnsupportedChannelsSnafu { value: channels }
        );

        let min_value = data.read_u32()?;
        let max_value = data.read_u32()?;
        let _reserved = data.read_u32()?;

        let image_name = data.read_exact::<80>()?;
        let colormap = data.read_u32()?;

        let _padding = data.read_exact::<404>()?;

        Ok(SgiHeader {
            compression,
            bytes_per_pixel,
            dimension,
            width,
            height,
            channels,
            _min_value: min_value,
            _max_value: max_value,
            _image_name: image_name,
            _colormap: colormap,
        })
    }
}

#[derive(Default)]
pub struct SgiImageLoader;

impl SgiImageLoader {
    fn decode_rle<T: ReadExt + SeekExt>(
        &self, data: &mut T, header: &SgiHeader,
    ) -> Result<Vec<u8>, SgiError> {
        // Make our code less verbose
        let width = header.width as usize;
        let height = header.height as usize;
        let channels = header.channels as usize;
        let bytes_per_pixel = header.bytes_per_pixel as usize;

        // Read offset and length tables
        let table_size = height * channels;
        let mut offsets = vec![0u32; table_size];
        let mut lengths = vec![0u32; table_size];

        for offset in offsets.iter_mut() {
            *offset = data.read_u32()?;
        }

        for length in lengths.iter_mut() {
            *length = data.read_u32()?;
        }

        let total_size = height * width * channels * bytes_per_pixel;
        let mut channel_data = DataCursor::new(vec![0u8; total_size], Endian::Big);

        // Process each scanline for each channel
        for channel in 0..channels {
            for row in 0..height {
                let table_pos = channel * height + row;
                let offset = offsets[table_pos] as u64;
                let length = lengths[table_pos] as usize;

                // This is pretty rough, TODO: improve seek pattern?
                data.set_position(offset)?;
                let compressed = data.read_slice(length)?;
                let mut compressed = DataCursorRef::new(&compressed, Endian::Big);

                let scanline_size = width * bytes_per_pixel;
                let out_pos = channel * width * height * bytes_per_pixel + row * scanline_size;
                channel_data.set_position(out_pos as u64)?;

                while compressed.position()? < compressed.len()? {
                    let mut count = if header.bytes_per_pixel == 1 {
                        compressed.read_u8()? as usize
                    } else {
                        compressed.read_u16()? as usize
                    };

                    if count == 0 {
                        break;
                    }

                    let is_run = (count & 0x80) == 0;
                    count &= 0x7F;

                    if is_run {
                        // Repeat value count times
                        if header.bytes_per_pixel == 1 {
                            let value = compressed.read_u8()?;
                            for _ in 0..count {
                                channel_data.write_u8(value)?;
                            }
                        } else {
                            let value = compressed.read_u16()?;
                            for _ in 0..count {
                                channel_data.write_u16(value)?;
                            }
                        }
                    } else {
                        // Copy count values
                        if header.bytes_per_pixel == 1 {
                            for _ in 0..count {
                                channel_data.write_u8(compressed.read_u8()?)?;
                            }
                        } else {
                            for _ in 0..count {
                                channel_data.write_u16(compressed.read_u16()?)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(channel_data.into_inner().to_vec())
    }
}

impl AssetLoader for SgiImageLoader {
    type Asset = Image;
    type Error = SgiError;
    type Settings = ();

    async fn load(
        &self, reader: &mut dyn Reader, _settings: &Self::Settings, _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let mut cursor = DataCursor::new(bytes, Endian::Big);
        let header = SgiHeader::read(&mut cursor)?;

        // Helper variables
        let width = header.width as usize;
        let height = header.height as usize;
        let channels = header.channels as usize;
        let bytes_per_pixel = header.bytes_per_pixel as usize;
        let total_size = height * width * channels * bytes_per_pixel;

        // Read the image data
        let channel_data = if header.compression == 1 {
            self.decode_rle(&mut cursor, &header)?
        } else {
            let mut data = vec![0u8; total_size];
            cursor.read_length(&mut data)?;
            data
        };

        // Determine format based on actual image properties
        let format = match (header.bytes_per_pixel, header.channels) {
            (1, 1) => TextureFormat::R8Unorm,
            (1, 3) | (1, 4) => TextureFormat::Rgba8Unorm,
            (2, 1) => TextureFormat::R16Unorm,
            (2, 3) | (2, 4) => TextureFormat::Rgba16Unorm,
            _ => unreachable!(), // We've already validated these combinations
        };

        let dimension = match header.dimension {
            1 => TextureDimension::D1,
            2 => TextureDimension::D2,
            3 => TextureDimension::D3,
            _ => unreachable!(), // Already validated
        };

        // For RGB formats, we need to expand to RGBA
        let needs_expansion = header.channels == 3;
        let output_channels = if needs_expansion { 4 } else { channels };
        let output_size = width * height * output_channels * bytes_per_pixel;
        let mut output_data = vec![0u8; output_size];

        // Convert from planar to pixel format and flip vertically
        for y in 0..height {
            for x in 0..width {
                let dst_row = y * width * output_channels * bytes_per_pixel;
                let src_row = (height - 1 - y) * width * bytes_per_pixel;
                let dst_pixel = dst_row + x * output_channels * bytes_per_pixel;

                // Copy existing channels (RGB or single channel)
                for c in 0..channels {
                    let src_pixel =
                        channels * src_row + c * width * height * bytes_per_pixel + x * bytes_per_pixel;
                    for b in 0..bytes_per_pixel {
                        output_data[dst_pixel + c * bytes_per_pixel + b] = channel_data[src_pixel + b];
                    }
                }

                // If we're expanding RGB to RGBA, set alpha to full opacity
                if needs_expansion {
                    let alpha_offset = dst_pixel + 3 * bytes_per_pixel;
                    for b in 0..bytes_per_pixel {
                        output_data[alpha_offset + b] = 0xFF;
                    }
                }
            }
        }

        Ok(Image::new(
            Extent3d { width: header.width as u32, height: header.height as u32, depth_or_array_layers: 1 },
            dimension,
            output_data,
            format,
            RenderAssetUsages::default(),
        ))
    }

    fn extensions(&self) -> &[&str] {
        &["sgi", "rgb", "rgba", "bw", "int", "inta"]
    }
}
