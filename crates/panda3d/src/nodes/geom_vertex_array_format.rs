use super::geom_vertex_column::GeomVertexColumn;
use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexArrayFormat {
    stride: u16,
    total_bytes: u16,
    pad_to: u8,
    divisor: u16,
    num_columns: u16,
    columns: Vec<GeomVertexColumn>,
}

impl GeomVertexArrayFormat {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let stride = data.read_u16()?;
        let total_bytes = data.read_u16()?;
        let pad_to = data.read_u8()?;
        let divisor = match loader.get_minor_version() > 36 {
            true => data.read_u16()?,
            false => 0,
        };
        let num_columns = data.read_u16()?;

        let mut format = Self {
            stride,
            total_bytes,
            pad_to,
            divisor,
            num_columns,
            ..Default::default()
        };

        for _ in 0..num_columns {
            format.columns.push(GeomVertexColumn::create(loader, data)?);
        }

        Ok(format)
    }
}
