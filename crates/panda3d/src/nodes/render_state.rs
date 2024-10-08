use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct RenderState {
    /// This stores a pointer to each RenderAttrib and its associated override value
    pub attrib_refs: Vec<(u32, i32)>,
}

impl RenderState {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_attribs = data.read_u16()?;
        let mut attrib_refs = Vec::with_capacity(num_attribs as usize);
        for _ in 0..num_attribs {
            let attrib_ref = loader.read_pointer(data)?.unwrap();
            let priority = data.read_i32()?;
            attrib_refs.push((attrib_ref, priority));
        }
        //TODO: in complete_pointers, we set the override attrib on each RenderAttrib
        Ok(Self { attrib_refs })
    }
}
