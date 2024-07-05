use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CollisionPlane {
    solid: CollisionSolid,
    plane: Vec4,
}

impl CollisionPlane {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let solid = CollisionSolid::create(loader, data)?;
        let plane = Vec4::read(data)?;
        Ok(Self { solid, plane })
    }
}
