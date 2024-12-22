use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomNode {
    /// ModelNode is a superclass of PandaNode, so we include its data here
    pub inner: PandaNode,
    /// Each piece of Geom data and its associated RenderState
    pub geom_refs: Vec<(u32, u32)>,
}

impl Node for GeomNode {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PandaNode::create(loader, data)?;

        //Cycler data
        let num_geoms = data.read_u16()?;
        let mut geom_refs = Vec::with_capacity(num_geoms as usize);
        for _ in 0..num_geoms {
            let geom_ref = loader.read_pointer(data)?.unwrap(); //Geom
            let render_ref = loader.read_pointer(data)?.unwrap(); //RenderState
            geom_refs.push((geom_ref, render_ref));
        }

        Ok(Self { inner, geom_refs })
    }
}

impl GraphDisplay for GeomNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{GeomNode|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        for reference in &self.geom_refs {
            connections.push(reference.0);
            connections.push(reference.1);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for GeomNode {
    type Target = PandaNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for GeomNode {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
