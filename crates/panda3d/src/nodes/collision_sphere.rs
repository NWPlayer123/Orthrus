use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CollisionSphere {
    solid: CollisionSolid,
    center: Vec3,
    radius: f32,
}

impl CollisionSphere {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let solid = CollisionSolid::create(loader, data)?;
        let center = Vec3::read(data)?;
        let radius = data.read_float()?;
        Ok(Self { solid, center, radius })
    }
}
