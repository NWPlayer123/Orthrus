use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct CullBinAttrib {
    bin_name: String,
    draw_order: i32,
}

impl CullBinAttrib {
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let bin_name = data.read_string()?;
        let draw_order = data.read_i32()?;
        Ok(Self { bin_name, draw_order })
    }
}
