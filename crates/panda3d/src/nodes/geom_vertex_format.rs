use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexFormat {
    pub animation: GeomVertexAnimationSpec,
    /// References to all GeomVertexArrayFormat data
    pub array_refs: Vec<u32>,
}

impl Node for GeomVertexFormat {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let animation = GeomVertexAnimationSpec::create(loader, data)?;

        let num_arrays = data.read_u16()?;
        let mut array_refs = Vec::with_capacity(num_arrays as usize);
        for _ in 0..num_arrays {
            array_refs.push(loader.read_pointer(data)?.unwrap());
        }

        Ok(Self { animation, array_refs })
    }
}

impl GraphDisplay for GeomVertexFormat {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{GeomVertexFormat|")?;
        }

        // Fields
        self.animation.write_data(label, connections, false)?;
        for reference in &self.array_refs {
            connections.push(*reference);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

// These aren't traditional inheritance but for the sake of the API, I'm making this a Deref
impl Deref for GeomVertexFormat {
    type Target = GeomVertexAnimationSpec;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.animation
    }
}

impl DerefMut for GeomVertexFormat {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.animation
    }
}
