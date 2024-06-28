use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum ColorType {
    #[default]
    Vertex,
    Flat,
    Off,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct ColorAttrib {
    pub color_type: ColorType,
    pub color: Vec4,
}

impl ColorAttrib {
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let color_type = ColorType::from(data.read_u8()?);
        //TODO: create custom color type?
        let color = Vec4::read(data)?;

        let mut attrib = Self { color_type, color };
        attrib.quantize_color();

        Ok(attrib)
    }

    fn quantize_color(&mut self) {
        match self.color_type {
            ColorType::Vertex => {
                self.color = Vec4::ZERO;
            }
            ColorType::Flat => {
                const SCALE: f32 = 1024.0;
                self.color = (self.color * SCALE + Vec4::splat(0.5)).floor() / SCALE;
            }
            ColorType::Off => {
                self.color = Vec4::ONE;
            }
        }
    }
}
