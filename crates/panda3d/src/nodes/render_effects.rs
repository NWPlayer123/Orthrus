use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct RenderEffects {
    /// References to all Effects
    pub effects: Vec<u32>,
}

impl RenderEffects {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_effects = data.read_u16()?;
        let mut effects = Vec::with_capacity(num_effects as usize);
        for _ in 0..num_effects {
            let effect = loader.read_pointer(data)?.unwrap();
            effects.push(effect);
        }

        Ok(Self { effects })
    }
}
