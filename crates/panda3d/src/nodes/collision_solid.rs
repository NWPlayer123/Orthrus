use super::prelude::*;

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub(crate) struct Flags: u8 {
        const Tangible = 1 << 0;
        const EffectiveNormal = 1 << 1;
        const VisibleGeometryStale = 1 << 2;
        const IgnoreEffectiveNormal = 1 << 3;
        const InternalBoundsStale = 1 << 4;
    }
}

#[derive(Debug, Default)]
pub(crate) struct CollisionSolid {
    pub flags: Flags,
    pub effective_normal: Vec3,
}

impl CollisionSolid {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let mut solid = Self::default();

        solid.flags = Flags::from_bits_truncate(data.read_u8()?);
        if solid.flags.contains(Flags::EffectiveNormal) {
            solid.effective_normal = Vec3::read(data)?;
        }

        solid.flags |= Flags::VisibleGeometryStale | Flags::InternalBoundsStale;

        Ok(solid)
    }
}
