use super::prelude::*;

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum PandaObject {
    BillboardEffect(BillboardEffect),
    Geom(Geom),
    GeomNode(GeomNode),
    GeomTristrips(GeomTristrips),
    GeomVertexData(GeomVertexData),
    ModelNode(ModelNode),
    PandaNode(PandaNode),
    RenderEffects(RenderEffects),
    RenderState(RenderState),
    TextureAttrib(TextureAttrib),
    TransformState(TransformState),
    TransparencyAttrib(TransparencyAttrib),
}
