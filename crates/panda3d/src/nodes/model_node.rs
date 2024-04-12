use super::panda_node::PandaNode;
use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum PreserveTransform {
    #[default]
    None,
    Local,
    Net,
    DropNode,
    NoTouch,
}

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct ModelNode {
    node: PandaNode,
    transform: PreserveTransform,
    attributes: u16,
}

impl ModelNode {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        let transform = PreserveTransform::from(data.read_u8()?);
        let attributes = data.read_u16()?;

        Ok(Self {
            node,
            transform,
            attributes,
        })
    }
}
