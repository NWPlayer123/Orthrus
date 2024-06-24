//! This module is for shared datatypes from the Panda3D codebase.

use core::ops::{Deref, DerefMut};

use orthrus_core::prelude::*;

/// This struct is mainly for readability in place of an unnamed tuple
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

impl core::fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

pub(crate) struct Datagram<'a> {
    data: DataCursorRef<'a>,
    float_type: bool,
}

impl<'a> Datagram<'a> {
    #[inline]
    pub(crate) fn new<T: DataCursorTrait + EndianRead>(
        data: &'a mut T, endian: Endian, float_type: bool,
    ) -> Result<Self, data::Error> {
        let length = data.read_u32()? as usize;
        Ok(Self {
            data: DataCursorRef::new(data.get_slice(length)?, endian),
            float_type,
        })
    }

    pub(crate) fn read_string(&mut self) -> Result<String, data::Error> {
        let length = self.data.read_u16()?;
        let slice = self.data.get_slice(length.into())?;
        let string = core::str::from_utf8(slice).map_err(|_| data::Error::InvalidUtf8)?;
        Ok(string.to_owned())
    }

    
    pub(crate) fn read_float(&mut self) -> Result<f64, data::Error> {
        if self.float_type == true {
            self.data.read_f64()
        } else {
            Ok(self.data.read_f32()? as f64)
        }
    }

    pub(crate) fn read_bool(&mut self) -> Result<bool, data::Error> {
        Ok(self.data.read_u8()? != 0)
    }
}

impl<'a> Deref for Datagram<'a> {
    type Target = DataCursorRef<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a> DerefMut for Datagram<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
