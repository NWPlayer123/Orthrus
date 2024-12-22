use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct MovingPartBase {
    pub inner: PartGroup,
    pub forced_channel_ref: Option<u32>,
}

impl MovingPartBase {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PartGroup::create(loader, data)?;

        let forced_channel_ref = match loader.get_minor_version() >= 20 {
            true => loader.read_pointer(data)?,
            false => None,
        };

        Ok(Self { inner, forced_channel_ref })
    }
}

impl GraphDisplay for MovingPartBase {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{MovingPartBase|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        if let Some(reference) = self.forced_channel_ref {
            connections.push(reference);
        }

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for MovingPartBase {
    type Target = PartGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MovingPartBase {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
