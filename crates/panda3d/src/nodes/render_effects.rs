use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct RenderEffects {
    /// References to all Effects
    pub effect_refs: Vec<u32>,
}

impl Node for RenderEffects {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_effects = data.read_u16()?;
        let mut effect_refs = Vec::with_capacity(num_effects as usize);
        for _ in 0..num_effects {
            effect_refs.push(loader.read_pointer(data)?.unwrap());
        }

        Ok(Self { effect_refs })
    }
}

impl Deref for RenderEffects {
    type Target = Vec<u32>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.effect_refs
    }
}

impl DerefMut for RenderEffects {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.effect_refs
    }
}
