use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub(crate) struct TransformEntry {
    pub transform_ref: u32,
    pub weight: f32,
}

impl TransformEntry {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let transform_ref = loader.read_pointer(data)?.unwrap();
        let weight = data.read_float()?;
        Ok(Self { transform_ref, weight })
    }
}

#[derive(Debug, Default)]
pub(crate) struct TransformBlend {
    pub entries: Vec<TransformEntry>,
}

impl TransformBlend {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_entries = data.read_u16()?;
        let mut entries = Vec::with_capacity(num_entries as usize);
        for _ in 0..num_entries {
            entries.push(TransformEntry::create(loader, data)?);
        }
        Ok(Self { entries })
    }
}

impl Deref for TransformBlend {
    type Target = Vec<TransformEntry>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for TransformBlend {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}
