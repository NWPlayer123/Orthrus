use crate::common::Datagram;
use crate::nodes::prelude::*;
use crate::bam::BinaryAsset;
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub(crate) trait DatagramRead {
    fn finalize(&self) -> Result<(), crate::bam::Error>;
}

#[enum_dispatch]
pub(crate) trait DatagramWrite {
    fn write(&self) -> Result<Datagram, crate::bam::Error>;
}

#[enum_dispatch(DatagramRead, DatagramWrite)]
enum Node {
    TextureStage(TextureStage),
    GeomVertexAnimationSpec(GeomVertexAnimationSpec),
}
