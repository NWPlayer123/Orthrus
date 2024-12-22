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

impl GraphDisplay for GeomVertexArrayFormat {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{GeomVertexArrayFormat|")?;
        }

        // Fields
        write!(label, "stride: {:#06X}|", self.stride)?;
        write!(label, "total_bytes: {:#06X}|", self.total_bytes)?;
        write!(label, "pad_to: {:#04X}|", self.pad_to)?;
        write!(label, "divisor: {:#06X}|", self.divisor)?;
        write!(label, "num_columns: {:#06X}|", self.num_columns)?;
        write!(label, "{{columns|")?;
        let mut first = true;
        for column in &self.columns {
            if !first {
                write!(label, "|")?;
            }
            column.write_data(label, connections, false)?;
            first = false;
        }
        write!(label, "}}")?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}
