use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct GeomNode {
    /// ModelNode is a superclass of a PandaNode, so we include its data here
    pub node: PandaNode,
    /// Each piece of Geom data and its associated RenderState
    pub geom_refs: Vec<(u32, u32)>,
}

impl GeomNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        //Cycler data
        let num_geoms = data.read_u16()?;
        let mut geom_refs = Vec::with_capacity(num_geoms as usize);
        for _ in 0..num_geoms {
            let geom_ref = loader.read_pointer(data)?.unwrap(); //Geom
            let render_ref = loader.read_pointer(data)?.unwrap(); //RenderState
            geom_refs.push((geom_ref, render_ref));
        }
        Ok(Self { node, geom_refs })
    }
}
