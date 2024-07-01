use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TransformEntry {
    transform_ref: u32,
    weight: f32,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TransformBlend {
    entries: Vec<TransformEntry>,
}

impl TransformBlend {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_entries = data.read_u16()?;
        let mut entries = Vec::with_capacity(num_entries as usize);
        for _ in 0..num_entries {
            let transform_ref = loader.read_pointer(data)?.unwrap();
            let weight = data.read_float()?;
            entries.push(TransformEntry { transform_ref, weight });
        }
        Ok(Self { entries })
    }
}
