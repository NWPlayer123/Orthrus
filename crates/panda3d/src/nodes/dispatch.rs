use super::prelude::*;

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum PandaObject {
    BillboardEffect(BillboardEffect),
    CollisionCapsule(CollisionCapsule), //Called CollisionTube previously
    CollisionNode(CollisionNode),
    ColorAttrib(ColorAttrib),
    CullBinAttrib(CullBinAttrib),
    CullFaceAttrib(CullFaceAttrib),
    DepthWriteAttrib(DepthWriteAttrib),
    Geom(Geom),
    GeomNode(GeomNode),
    GeomTristrips(GeomTristrips),
    GeomVertexArrayData(GeomVertexArrayData),
    GeomVertexArrayFormat(GeomVertexArrayFormat),
    GeomVertexData(GeomVertexData),
    GeomVertexFormat(GeomVertexFormat),
    InternalName(InternalName),
    ModelNode(ModelNode),
    PandaNode(PandaNode),
    RenderEffects(RenderEffects),
    RenderState(RenderState),
    Texture(Texture),
    TextureAttrib(TextureAttrib),
    TextureStage(TextureStage),
    TransformState(TransformState),
    TransparencyAttrib(TransparencyAttrib),
    Null,
}
