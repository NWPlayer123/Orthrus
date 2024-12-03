//! This module is for shared datatypes from the Panda3D codebase.

use core::ops::{Deref, DerefMut};
use std::borrow::Cow;

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

// TODO: just make this a generic and enforce f32/f64 depending on the BAM file using a sealed trait like we
// do in Ferrox
pub(crate) struct Datagram<'a> {
    cursor: DataCursorRef<'a>,
    float_type: bool,
}

impl<'a> Datagram<'a> {
    #[inline]
    pub(crate) fn new<T: ReadExt>(
        data: &'a mut T, endian: Endian, float_type: bool,
    ) -> Result<Self, DataError> {
        let length = data.read_u32()? as usize;
        let data = match data.read_slice(length)? {
            Cow::Borrowed(data) => data,
            Cow::Owned(_) => todo!(),
        };
        Ok(Self { cursor: DataCursorRef::new(data, endian), float_type })
    }

    pub(crate) fn read_string(&mut self) -> Result<String, DataError> {
        let length = self.cursor.read_u16()?;
        let slice = self.cursor.read_slice(length.into())?;
        let string = core::str::from_utf8(&slice).map_err(|source| DataError::InvalidStr { source })?;
        Ok(string.to_owned())
    }

    pub(crate) fn read_float(&mut self) -> Result<f32, DataError> {
        match self.float_type {
            true => Ok(self.cursor.read_f64()? as f32),
            false => self.cursor.read_f32(),
        }
    }

    pub(crate) fn read_bool(&mut self) -> Result<bool, DataError> {
        Ok(self.cursor.read_u8()? != 0)
    }
}

impl<'a> Deref for Datagram<'a> {
    type Target = DataCursorRef<'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.cursor
    }
}

impl DerefMut for Datagram<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cursor
    }
}
