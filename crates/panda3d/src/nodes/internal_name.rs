use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
pub(crate) struct InternalName {
    pub name: String,
}

impl Node for InternalName {
    #[inline]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        Ok(Self { name: data.read_string()? })
    }
}

impl GraphDisplay for InternalName {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        write!(label, "{{InternalName|")?;

        // Fields
        write!(label, "name: {}", self.name)?;

        // Footer
        write!(label, "}}")?;
        Ok(())
    }
}

impl Deref for InternalName {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl DerefMut for InternalName {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.name
    }
}
