use super::prelude::*;

//TODO: This is technically a generic but I don't want to make it a generic right now
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct MovingPartMatrix {
    pub base: MovingPartBase,
    pub value: Mat4,
    pub default_value: Mat4,
}

impl MovingPartMatrix {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let base = MovingPartBase::create(loader, data)?;
        let value = Mat4::read(data)?;
        let default_value = Mat4::read(data)?;
        Ok(Self { base, value, default_value })
    }
}
