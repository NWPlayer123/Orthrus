use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct RenderState {
    /// This stores a pointer to each RenderAttrib and its associated override value
    pub attrib_refs: Vec<(u32, i32)>,
}

impl Node for RenderState {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
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

impl GraphDisplay for RenderState {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // This doesn't have any data, write a placeholder
        write!(label, "{{RenderState|count: {}}}", self.attrib_refs.len())?;
        for reference in &self.attrib_refs {
            connections.push(reference.0);
        }
        Ok(())
    }
}

impl Deref for RenderState {
    type Target = Vec<(u32, i32)>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.attrib_refs
    }
}

impl DerefMut for RenderState {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.attrib_refs
    }
}
