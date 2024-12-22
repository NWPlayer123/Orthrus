use core::ops::{Deref, DerefMut};

use super::prelude::*;

// This is technically a generic but I don't feel like making one
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AnimChannelMatrix {
    pub inner: AnimGroup,
    pub last_frame: u16,
}

impl AnimChannelMatrix {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = AnimGroup::create(loader, data)?;
        let last_frame = data.read_u16()?;
        Ok(Self { inner, last_frame })
    }
}

impl GraphDisplay for AnimChannelMatrix {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{AnimChannelMatrix|")?;
        }
        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(label, "|last_frame: {:#06X}", self.last_frame)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for AnimChannelMatrix {
    type Target = AnimGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AnimChannelMatrix {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
