use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexArrayFormat {
    pub stride: u16,
    pub total_bytes: u16,
    pub pad_to: u8,
    pub divisor: u16,
    pub num_columns: u16,
    pub columns: Vec<GeomVertexColumn>,
}

impl Node for GeomVertexArrayFormat {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let stride = data.read_u16()?;
        let total_bytes = data.read_u16()?;
        let pad_to = data.read_u8()?;
        let divisor = match loader.get_minor_version() > 36 {
            true => data.read_u16()?,
            false => 0,
        };

        let num_columns = data.read_u16()?;
        let mut columns = Vec::with_capacity(num_columns as usize);
        for _ in 0..num_columns {
            columns.push(GeomVertexColumn::create(loader, data)?);
        }

        Ok(Self { stride, total_bytes, pad_to, divisor, num_columns, columns })
    }
}
