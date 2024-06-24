use super::geom_primitive::GeomPrimitive;
use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct GeomTristrips {
    primitive: GeomPrimitive,
}

impl GeomTristrips {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { primitive: GeomPrimitive::create(loader, data)? })
    }
}
