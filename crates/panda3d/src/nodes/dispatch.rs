use super::prelude::*;

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum PandaObject {
    BillboardEffect(BillboardEffect),
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
}
