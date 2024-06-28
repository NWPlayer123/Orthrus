use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomNode {
    /// ModelNode is a superclass of a PandaNode, so we include its data here
    pub node: PandaNode,
    /// Each piece of Geom data and its associated RenderState
    pub geoms: Vec<(u32, u32)>,
}

impl GeomNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        //Cycler data
        let num_geoms = data.read_u16()?;
        let mut geoms = Vec::with_capacity(num_geoms as usize);
        for _ in 0..num_geoms {
            let geom = loader.read_pointer(data)?.unwrap(); //Geom
            let render = loader.read_pointer(data)?.unwrap(); //RenderState
            geoms.push((geom, render));
        }
        Ok(Self { node, geoms })
    }
}
