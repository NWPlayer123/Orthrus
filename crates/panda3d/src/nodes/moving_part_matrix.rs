use core::ops::{Deref, DerefMut};

use super::prelude::*;

//TODO: This is technically a generic but I don't want to make it a generic right now
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct MovingPartMatrix {
    pub inner: MovingPartBase,
    pub value: Mat4,
    pub default_value: Mat4,
}

impl MovingPartMatrix {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = MovingPartBase::create(loader, data)?;

        let value = Mat4::read(data)?;
        let default_value = Mat4::read(data)?;

        Ok(Self { inner, value, default_value })
    }
}

impl GraphDisplay for MovingPartMatrix {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{MovingPartMatrix|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(
            label,
            "|{{value|{}\\n{}\\n{}\\n{}}}",
            self.value.w_axis, self.value.x_axis, self.value.y_axis, self.value.z_axis
        )?;
        write!(
            label,
            "|{{default_value|{}\\n{}\\n{}\\n{}}}",
            self.default_value.w_axis,
            self.default_value.x_axis,
            self.default_value.y_axis,
            self.default_value.z_axis
        )?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for MovingPartMatrix {
    type Target = MovingPartBase;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MovingPartMatrix {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
