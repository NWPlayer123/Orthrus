use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CullBinAttrib {
    pub bin_name: String,
    pub draw_order: i32,
}

impl Node for CullBinAttrib {
    #[inline]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        let bin_name = data.read_string()?;
        let draw_order = data.read_i32()?;
        Ok(Self { bin_name, draw_order })
    }
}
