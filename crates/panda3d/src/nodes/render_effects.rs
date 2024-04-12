use super::prelude::*;

#[derive(Default, Debug)]
#[allow(dead_code)]
pub(crate) struct RenderEffects {
    effects: Vec<Option<u32>>,
}

impl RenderEffects {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        //Cycler data
        let num_effects = data.read_u16()?;
        //Effect*
        let mut effects = Vec::with_capacity(num_effects as usize);
        for _ in 0..num_effects {
            let effect = loader.read_pointer(data)?;
            effects.push(effect);
        }
        Ok(Self { effects })
    }
}
