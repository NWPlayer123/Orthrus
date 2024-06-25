use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomTristrips {
    primitive: GeomPrimitive,
}

impl GeomTristrips {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { primitive: GeomPrimitive::create(loader, data)? })
    }
}
