use super::prelude::*;

/// The PreserveTransform attribute tells us how a flatten operation can affect the transform data
/// on this node.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum PreserveTransform {
    #[default]
    /// No restrictions, the transform can be modified at-will.
    None,
    /// Preserve both the local and net transforms.
    Local,
    /// Preserve the net tranform of this object. Local transform is allowed to be modified.
    Net,
    /// Remove this node at the next flatten call.
    DropNode,
    /// This node and all children cannot be flattened, and the node will not be removed.
    NoTouch,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct ModelNode {
    /// ModelNode is a superclass of a PandaNode, so we include its data here
    node: PandaNode,
    /// Whether to preserve the PandaNode transform data.
    transform: PreserveTransform,
    // TODO: bitflag union from SceneGraphReducer::AttribTypes of which attributes to protect
    attributes: u16,
}

impl ModelNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        let transform = PreserveTransform::from(data.read_u8()?);
        let attributes = data.read_u16()?;

        Ok(Self { node, transform, attributes })
    }
}
