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
#[allow(dead_code)]
pub(crate) struct CollisionSolid {
    pub flags: Flags,
    pub effective_normal: Vec3,
}

impl CollisionSolid {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        let mut flags = Flags::from_bits_truncate(data.read_u8()?);

        let effective_normal = match flags.contains(Flags::EffectiveNormal) {
            true => Vec3::read(data)?,
            false => Vec3::default(),
        };

        flags |= Flags::VisibleGeometryStale | Flags::InternalBoundsStale;

        Ok(Self { flags, effective_normal })
    }
}
