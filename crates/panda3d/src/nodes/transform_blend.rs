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

impl GraphDisplay for TransformBlend {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        write!(label, "{{TransformBlend|")?;

        // Fields
        let mut first = true;
        for entry in &self.entries {
            if !first {
                write!(label, "|")?;
            }
            //connections.push(entry.transform_ref);
            write!(
                label,
                "{{transform: {}|weight: {}}}",
                entry.transform_ref, entry.weight
            )?;
            first = false;
        }

        // Footer
        write!(label, "}}")?;
        Ok(())
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
