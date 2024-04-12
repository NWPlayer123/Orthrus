use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum ColorType {
    #[default]
    Vertex,
    Flat,
    Off,
}

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct ColorAttrib {
    color_type: ColorType,
    color: [f64; 4],
}

impl ColorAttrib {
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let color_type = ColorType::from(data.read_u8()?);
        //LColor -> [f64; 4]
        let color = [
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
        ];

        let mut attrib = Self { color_type, color };
        attrib.quantize_color();

        Ok(attrib)
    }

    fn quantize_color(&mut self) {
        match self.color_type {
            ColorType::Vertex => {
                self.color = [0.0, 0.0, 0.0, 0.0];
            }
            ColorType::Flat => {
                //TODO: SIMD? once it's stabilized
                self.color[0] = f64::floor(self.color[0] * 1024.0 + 0.5) / 1024.0;
                self.color[1] = f64::floor(self.color[1] * 1024.0 + 0.5) / 1024.0;
                self.color[2] = f64::floor(self.color[2] * 1024.0 + 0.5) / 1024.0;
                self.color[3] = f64::floor(self.color[3] * 1024.0 + 0.5) / 1024.0;
            }
            ColorType::Off => {
                self.color = [1.0, 1.0, 1.0, 1.0];
            }
        }
    }
}
