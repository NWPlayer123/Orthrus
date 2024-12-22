use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct CharacterJointEffect {
    pub character_ref: u32,
}

impl Node for CharacterJointEffect {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(Self { character_ref: loader.read_pointer(data)?.unwrap() })
    }
}

impl GraphDisplay for CharacterJointEffect {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // This has no data, so let's just do one write
        write!(label, "{{CharacterJointEffect}}")?;
        connections.push(self.character_ref);
        Ok(())
    }
}

impl Deref for CharacterJointEffect {
    type Target = u32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.character_ref
    }
}

impl DerefMut for CharacterJointEffect {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.character_ref
    }
}
