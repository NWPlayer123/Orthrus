use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CharacterJointEffect {
    character_ref: u32,
}

impl CharacterJointEffect {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { character_ref: loader.read_pointer(data)?.unwrap() })
    }
}
