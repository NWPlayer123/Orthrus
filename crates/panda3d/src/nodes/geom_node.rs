use super::panda_node::PandaNode;
use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomNode {
    node: PandaNode,
    geoms: Vec<(Option<u32>, Option<u32>)>,
}

impl GeomNode {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        //Cycler data
        let num_geoms = data.read_u16()?;
        let mut geoms = Vec::with_capacity(num_geoms as usize);
        for _ in 0..num_geoms {
            let geom = loader.read_pointer(data)?; //Geom
            let render = loader.read_pointer(data)?; //RenderState
            geoms.push((geom, render));
        }
        Ok(Self { node, geoms })
    }
}
