use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct RenderEffects {
    /// References to all Effects
    pub effect_refs: Vec<u32>,
}

impl RenderEffects {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_effects = data.read_u16()?;
        let mut effect_refs = Vec::with_capacity(num_effects as usize);
        for _ in 0..num_effects {
            let effect_ref = loader.read_pointer(data)?.unwrap();
            effect_refs.push(effect_ref);
        }

        Ok(Self { effect_refs })
    }
}
