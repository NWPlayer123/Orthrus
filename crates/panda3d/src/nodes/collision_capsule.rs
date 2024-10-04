use super::collision_solid::Flags;
use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct CollisionCapsule {
    flags: Flags,
    effective_normal: Vec3,
    a: Vec3,
    b: Vec3,
    radius: f32,
    length: f32,
}

impl CollisionCapsule {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let solid = CollisionSolid::create(loader, data)?;

        let a = Vec3::read(data)?;
        let b = Vec3::read(data)?;
        let radius = data.read_float()?;

        let mut capsule = Self {
            flags: solid.flags,
            effective_normal: solid.effective_normal,
            a,
            b,
            radius,
            ..Default::default()
        };

        capsule.recalc_internals();

        Ok(capsule)
    }

    fn recalc_internals(&mut self) {
        self.length = (self.b - self.a).length();
        //TODO: calculate the matrix
    }
}
