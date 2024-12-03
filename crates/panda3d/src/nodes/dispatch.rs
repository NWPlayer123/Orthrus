use super::prelude::*;

pub trait Node: std::fmt::Debug + Send + Sync {
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error>
    where
        Self: Sized;
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum PandaObject {
    AnimBundle(AnimBundle),
    AnimBundleNode(AnimBundleNode),
    AnimChannelMatrixXfmTable(AnimChannelMatrixXfmTable),
    AnimGroup(AnimGroup),
    BillboardEffect(BillboardEffect),
    Character(Character),
    CharacterJoint(CharacterJoint),
    CharacterJointBundle(PartBundle),
    CharacterJointEffect(CharacterJointEffect),
    CollisionCapsule(CollisionCapsule), //Called CollisionTube previously
    CollisionNode(CollisionNode),
    CollisionPolygon(CollisionPolygon),
    CollisionSphere(CollisionSphere),
    ColorAttrib(ColorAttrib),
    CullBinAttrib(CullBinAttrib),
    CullFaceAttrib(CullFaceAttrib),
    DecalEffect(DecalEffect),
    DepthWriteAttrib(DepthWriteAttrib),
    Geom(Geom),
    GeomNode(GeomNode),
    GeomTriangles(GeomPrimitive),
    GeomTristrips(GeomPrimitive),
    GeomVertexArrayData(GeomVertexArrayData),
    GeomVertexArrayFormat(GeomVertexArrayFormat),
    GeomVertexData(GeomVertexData),
    GeomVertexFormat(GeomVertexFormat),
    InternalName(InternalName),
    JointVertexTransform(JointVertexTransform),
    LODNode(LODNode),
    ModelNode(ModelNode),
    ModelRoot(ModelNode),
    PandaNode(PandaNode),
    PartGroup(PartGroup),
    RenderEffects(RenderEffects),
    RenderState(RenderState),
    Texture(Texture),
    TextureAttrib(TextureAttrib),
    TextureStage(TextureStage),
    TransformBlendTable(TransformBlendTable),
    TransformState(TransformState),
    TransparencyAttrib(TransparencyAttrib),
    Null,
}
