use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct RenderState {
    attribs: Vec<(Option<u32>, i32)>,
}

impl RenderState {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_attribs = data.read_u16()?;
        let mut attribs = Vec::with_capacity(num_attribs as usize);
        for _ in 0..num_attribs {
            let attrib = loader.read_pointer(data)?;
            let priority = data.read_i32()?;
            attribs.push((attrib, priority));
        }
        Ok(Self { attribs })
    }
}
