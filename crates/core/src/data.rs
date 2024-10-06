use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

use snafu::prelude::*;

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
#[cfg(feature = "std")]
use std::{
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    path::Path,
};

/// Error conditions for when reading/writing data.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum DataError {
    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Tried to read out-of-bounds"))]
    EndOfFile,
    /// Thrown if UTF-8 validation fails when trying to convert a slice.
    #[snafu(display("Invalid UTF-8 sequence"))]
    InvalidStr { source: core::str::Utf8Error },
    /// Thrown if UTF-8 validation fails when trying to convert a slice.
    #[snafu(display("Invalid UTF-8 sequence"))]
    InvalidString { source: alloc::string::FromUtf8Error },
    /// Thrown when an I/O operation fails on a [`DataStream`].
    #[cfg(feature = "std")]
    #[snafu(display("I/O error: {}", source))]
    Io { source: std::io::Error },
}

/// Represents the endianness of the data being read or written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Endian {
    Little,
    Big,
}

impl Default for Endian {
    #[inline]
    fn default() -> Self {
        #[cfg(target_endian = "little")]
        {
            Endian::Little
        }
        #[cfg(target_endian = "big")]
        {
            Endian::Big
        }
    }
}

/// Trait for types that support endian-aware operations.
pub trait EndianExt {
    /// Returns the current endianness.
    fn endian(&self) -> Endian;

    /// Sets the endianness.
    fn set_endian(&mut self, endian: Endian);
}

/// Trait for types that support seeking operations.
pub trait SeekExt {
    /// Returns the current position.
    fn position(&mut self) -> usize;

    /// Sets the current position.
    ///
    /// # Errors
    /// Returns an error if the position cannot be set.
    fn set_position(&mut self, position: usize) -> Result<usize, DataError>;

    /// Returns the total length of the data.
    ///
    /// # Errors
    /// Returns an error if unable to determine the length of the stream.
    fn len(&mut self) -> Result<usize, DataError>;

    /// Returns `true` if the remaining data is empty.
    ///
    /// # Errors
    /// Returns an error if unable to determine either the length of the stream or the position inside it.
    #[inline]
    fn is_empty(&mut self) -> Result<bool, DataError> {
        Ok(self.len()?.saturating_sub(self.position()) == 0)
    }
}

/// Trait for types that support reading operations.
pub trait ReadExt: EndianExt {
    /// Reads exactly N bytes from the current stream.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DataError>;

    /// Attempts to fill the buffer with data.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_length(&mut self, buffer: &mut [u8]) -> Result<usize, DataError>;

    /// Reads a slice of the given length from the current position.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[cfg(not(feature = "alloc"))]
    fn read_slice(&mut self, length: usize) -> Result<&[u8], DataError>;

    /// Reads a slice of the given length from the current position.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[cfg(feature = "alloc")]
    fn read_slice(&mut self, length: usize) -> Result<Cow<[u8]>, DataError>;

    /// Reads a UTF-8 encoded string of the given length from the current position.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    /// Returns [`InvalidStr`](Error::InvalidStr) if the bytes are not valid UTF-8.
    #[inline]
    #[cfg(not(feature = "alloc"))]
    fn read_string(&mut self, length: usize) -> Result<&str, DataError> {
        let slice = self.read_slice(length)?;
        core::str::from_utf8(slice).context(InvalidStrSnafu)
    }

    /// Returns the remaining data from the current position.
    ///
    /// # Errors
    /// Returns an error if the remaining data cannot be read.
    #[cfg(not(feature = "alloc"))]
    fn remaining_slice(&mut self) -> Result<&[u8], DataError>;

    /// Returns the remaining data from the current position.
    ///
    /// # Errors
    /// Returns an error if the remaining data cannot be read.
    #[cfg(feature = "alloc")]
    fn remaining_slice(&mut self) -> Result<Cow<[u8]>, DataError>;

    /// Reads a UTF-8 encoded string of the given length from the current position.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    /// Returns [`InvalidStr`](Error::InvalidStr) if the bytes are not valid UTF-8.
    #[inline]
    #[cfg(feature = "alloc")]
    fn read_string(&mut self, length: usize) -> Result<Cow<str>, DataError> {
        let slice = self.read_slice(length)?;
        match slice {
            Cow::Borrowed(bytes) => core::str::from_utf8(bytes).map(Cow::Borrowed).context(InvalidStrSnafu),
            Cow::Owned(bytes) => String::from_utf8(bytes).map(Cow::Owned).context(InvalidStringSnafu),
        }
    }

    /// Reads an unsigned 8-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_u8(&mut self) -> Result<u8, DataError> {
        Ok(self.read_exact::<1>()?[0])
    }

    /// Reads a signed 8-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_i8(&mut self) -> Result<i8, DataError> {
        Ok(self.read_u8()? as i8)
    }

    /// Reads an unsigned 16-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_u16(&mut self) -> Result<u16, DataError> {
        let bytes = self.read_exact()?;
        Ok(match self.endian() {
            Endian::Little => u16::from_le_bytes(bytes),
            Endian::Big => u16::from_be_bytes(bytes),
        })
    }

    /// Reads a signed 16-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_i16(&mut self) -> Result<i16, DataError> {
        Ok(self.read_u16()? as i16)
    }

    /// Reads an unsigned 32-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_u32(&mut self) -> Result<u32, DataError> {
        let bytes = self.read_exact()?;
        Ok(match self.endian() {
            Endian::Little => u32::from_le_bytes(bytes),
            Endian::Big => u32::from_be_bytes(bytes),
        })
    }

    /// Reads a signed 32-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_i32(&mut self) -> Result<i32, DataError> {
        Ok(self.read_u32()? as i32)
    }

    /// Reads an unsigned 64-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_u64(&mut self) -> Result<u64, DataError> {
        let bytes = self.read_exact()?;
        Ok(match self.endian() {
            Endian::Little => u64::from_le_bytes(bytes),
            Endian::Big => u64::from_be_bytes(bytes),
        })
    }

    /// Reads a signed 64-bit integer.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_i64(&mut self) -> Result<i64, DataError> {
        Ok(self.read_u64()? as i64)
    }

    /// Reads a 32-bit floating point number.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_f32(&mut self) -> Result<f32, DataError> {
        let bytes = self.read_exact()?;
        Ok(match self.endian() {
            Endian::Little => f32::from_le_bytes(bytes),
            Endian::Big => f32::from_be_bytes(bytes),
        })
    }

    /// Reads a 64-bit floating point number.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    #[inline]
    fn read_f64(&mut self) -> Result<f64, DataError> {
        let bytes = self.read_exact()?;
        Ok(match self.endian() {
            Endian::Little => f64::from_le_bytes(bytes),
            Endian::Big => f64::from_be_bytes(bytes),
        })
    }
}

/// Trait for types that support writing operations.
pub trait WriteExt: EndianExt {
    /// Writes exactly N bytes to the current stream.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    fn write_exact<const N: usize>(&mut self, bytes: &[u8; N]) -> Result<(), DataError>;

    /// Writes an unsigned 8-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_u8(&mut self, value: u8) -> Result<(), DataError> {
        self.write_exact(&[value])
    }

    /// Writes a signed 8-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_i8(&mut self, value: i8) -> Result<(), DataError> {
        self.write_u8(value as u8)
    }

    /// Writes an unsigned 16-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_u16(&mut self, value: u16) -> Result<(), DataError> {
        let bytes = match self.endian() {
            Endian::Little => value.to_le_bytes(),
            Endian::Big => value.to_be_bytes(),
        };
        self.write_exact(&bytes)
    }

    /// Writes a signed 16-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_i16(&mut self, value: i16) -> Result<(), DataError> {
        self.write_u16(value as u16)
    }

    /// Writes an unsigned 32-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_u32(&mut self, value: u32) -> Result<(), DataError> {
        let bytes = match self.endian() {
            Endian::Little => value.to_le_bytes(),
            Endian::Big => value.to_be_bytes(),
        };
        self.write_exact(&bytes)
    }

    /// Writes a signed 32-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_i32(&mut self, value: i32) -> Result<(), DataError> {
        self.write_u32(value as u32)
    }

    /// Writes an unsigned 64-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_u64(&mut self, value: u64) -> Result<(), DataError> {
        let bytes = match self.endian() {
            Endian::Little => value.to_le_bytes(),
            Endian::Big => value.to_be_bytes(),
        };
        self.write_exact(&bytes)
    }

    /// Writes a signed 64-bit integer.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_i64(&mut self, value: i64) -> Result<(), DataError> {
        self.write_u64(value as u64)
    }

    /// Writes a 32-bit floating point number.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_f32(&mut self, value: f32) -> Result<(), DataError> {
        let bytes = match self.endian() {
            Endian::Little => value.to_le_bytes(),
            Endian::Big => value.to_be_bytes(),
        };
        self.write_exact(&bytes)
    }

    /// Writes a 64-bit floating point number.
    ///
    /// # Errors
    /// Returns an error if the write operation fails.
    #[inline]
    fn write_f64(&mut self, value: f64) -> Result<(), DataError> {
        let bytes = match self.endian() {
            Endian::Little => value.to_le_bytes(),
            Endian::Big => value.to_be_bytes(),
        };
        self.write_exact(&bytes)
    }
}

/// An owned, in-memory file that allows endian-aware read and write.
///
/// This is architected to assume a fixed length, and is `no_std` compatible.
#[derive(Debug)]
pub struct DataCursor {
    data: Box<[u8]>,
    position: usize,
    endian: Endian,
}

impl DataCursor {
    /// Creates a new `DataCursor` with the given data and endianness.
    #[inline]
    pub fn new<I: Into<Box<[u8]>>>(data: I, endian: Endian) -> Self {
        Self { data: data.into(), position: 0, endian }
    }

    /// Creates a new `DataCursor` with the given path and endianness.
    ///
    /// # Errors
    /// Returns an error if the file does not exist or is unable to be opened.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_path<P: AsRef<Path>>(path: P, endian: Endian) -> std::io::Result<Self> {
        Ok(Self::new(std::fs::read(path)?, endian))
    }

    /// Consumes the `DataCursor` and returns the underlying data.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> Box<[u8]> {
        self.data
    }

    /// Shrinks the underlying data to the new length and returns the modified `DataCursor`.
    #[inline]
    #[must_use]
    pub fn shrink_to(mut self, new_len: usize) -> Self {
        // If the user tries to expand, just keep the current length.
        if new_len < self.data.len() {
            // Otherwise, modify the current buffer to drop all data past the desired length.
            self.data = self.data[..new_len].into();
            // Make sure our new position is within the bounds!
            if self.position > new_len {
                self.position = new_len;
            }
        }
        self
    }

    /// Copies data from this `DataCursor` to another mutable slice.
    #[inline]
    pub fn copy_data_to(&self, other: &mut [u8]) {
        let len = self.data.len().min(other.len());
        // SAFETY: We have a valid length, other cannot overlap self since there's no way to acquire a mutable
        // reference, and we will always have a valid alignment.
        unsafe {
            core::ptr::copy_nonoverlapping(self.data.as_ptr(), other.as_mut_ptr(), len);
        }
    }

    /// Copies data within the `DataCursor` from one range to another position.
    ///
    /// # Example
    /// ```
    /// # use orthrus_core::prelude::*;
    /// let mut cursor = DataCursor::new(vec![1, 2, 3, 4, 5].into_boxed_slice(), Endian::Little);
    /// cursor.copy_within(1..4, 2).unwrap();
    /// assert_eq!(&cursor.into_inner()[..], &[1, 2, 2, 3, 4]);
    /// ```
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if either the source range or the destination
    /// range would be out of bounds.
    #[inline]
    pub fn copy_within(&mut self, src: core::ops::Range<usize>, dest: usize) -> Result<(), DataError> {
        let length = src.end.saturating_sub(src.start);
        ensure!(
            src.end <= self.data.len() && dest.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        if src.contains(&dest) {
            for i in 0..length {
                // SAFETY: We want specific behavior if the ranges overlap, due to how Yaz0 compression works.
                // Both ranges are within bounds and have a valid alignment.
                unsafe {
                    *self.data.as_mut_ptr().add(dest.saturating_add(i)) =
                        *self.data.as_ptr().add(src.start.saturating_add(i));
                }
            }
        } else {
            // SAFETY: Both ranges are within bounds, do not overlap, and have a valid alignment.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.data.as_ptr().add(src.start),
                    self.data.as_mut_ptr().add(dest),
                    length,
                );
            }
        }
        Ok(())
    }
}

impl EndianExt for DataCursor {
    #[inline]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }
}

impl SeekExt for DataCursor {
    #[inline]
    fn position(&mut self) -> usize {
        self.position
    }

    #[inline]
    fn set_position(&mut self, position: usize) -> Result<usize, DataError> {
        self.position = position.min(self.data.len());
        Ok(self.position)
    }

    #[inline]
    fn len(&mut self) -> Result<usize, DataError> {
        Ok(self.data.len())
    }
}

impl ReadExt for DataCursor {
    #[inline]
    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DataError> {
        ensure!(self.position.saturating_add(N) <= self.data.len(), EndOfFileSnafu);

        let mut result: MaybeUninit<[u8; N]> = MaybeUninit::uninit();
        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment. We use
        // MaybeUninit here to skip some overhead when we immediately overwrite it with new data.
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.position),
                result.as_mut_ptr().cast(),
                N,
            );
        }
        self.position = self.position.saturating_add(N);
        // SAFETY: We've initialized this data, so this is safe.
        Ok(unsafe { result.assume_init() })
    }

    #[inline]
    fn read_length(&mut self, buffer: &mut [u8]) -> Result<usize, DataError> {
        let length = buffer.len().min(self.data.len().saturating_sub(self.position));

        // SAFETY: We're within the bounds of both `buf` and `self.data`, and will always have a valid
        // alignment. There is no way to get a mutable reference to the inner data, so buffer cannot overlap.
        unsafe {
            let src_ptr = self.data.as_ptr().add(self.position);
            core::ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), length);
        }
        self.position = self.position.saturating_add(length);
        Ok(length)
    }

    #[inline]
    #[cfg(not(feature = "alloc"))]
    fn read_slice(&mut self, length: usize) -> Result<&[u8], DataError> {
        ensure!(
            self.position.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, length)
        };
        self.position += length;
        Ok(result)
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn read_slice(&mut self, length: usize) -> Result<Cow<[u8]>, DataError> {
        ensure!(
            self.position.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, length)
        };
        self.position = self.position.saturating_add(length);
        Ok(Cow::Borrowed(result))
    }

    #[inline]
    #[cfg(not(feature = "alloc"))]
    fn remaining_slice(&mut self) -> Result<&[u8], DataError> {
        // SAFETY: We're within bounds since we're reading to the end, and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, self.data.len().saturating_sub(self.position))
        };
        self.position = self.data.len();
        Ok(result)
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn remaining_slice(&mut self) -> Result<Cow<[u8]>, DataError> {
        // SAFETY: We're within bounds since we're reading to the end, and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, self.data.len().saturating_sub(self.position))
        };
        self.position = self.data.len();
        Ok(Cow::Borrowed(result))
    }
}

impl WriteExt for DataCursor {
    #[inline]
    fn write_exact<const N: usize>(&mut self, bytes: &[u8; N]) -> Result<(), DataError> {
        ensure!(self.position.saturating_add(N) <= self.data.len(), EndOfFileSnafu);

        // SAFETY: We're within the bounds of `self.data`, `bytes` will always be valid, and we'll always have
        // a valid alignment.
        unsafe {
            let dst_ptr = self.data.as_mut_ptr().add(self.position);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst_ptr, N);
        }
        self.position = self.position.saturating_add(N);
        Ok(())
    }
}

impl From<Box<[u8]>> for DataCursor {
    #[inline]
    fn from(value: Box<[u8]>) -> Self {
        Self { data: value, position: 0, endian: Endian::default() }
    }
}

#[cfg(feature = "std")]
impl From<Vec<u8>> for DataCursor {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self { data: value.into_boxed_slice(), position: 0, endian: Endian::default() }
    }
}

impl Deref for DataCursor {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for DataCursor {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl AsMut<[u8]> for DataCursor {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// A borrowed, in-memory file that allows endian-aware read.
///
/// This is architected to assume a fixed length, and is `no_std` compatible.
#[derive(Debug)]
pub struct DataCursorRef<'a> {
    data: &'a [u8],
    position: usize,
    endian: Endian,
}

impl<'a> DataCursorRef<'a> {
    /// Creates a new `DataCursorRef` with the given data and endianness.
    #[inline]
    #[must_use]
    pub fn new(data: &'a [u8], endian: Endian) -> Self {
        Self { data, position: 0, endian }
    }

    /// Consumes the `DataCursorRef` and returns the underlying data.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> &'a [u8] {
        self.data
    }

    /// Copies data from this `DataCursorRef` to a mutable slice.
    #[inline]
    pub fn copy_data_to(&self, other: &mut [u8]) {
        let len = self.data.len().min(other.len());
        // SAFETY: We have a valid length, other cannot overlap self since there's no way to acquire a mutable
        // reference, and we will always have a valid alignment.
        unsafe {
            core::ptr::copy_nonoverlapping(self.data.as_ptr(), other.as_mut_ptr(), len);
        }
    }
}

impl EndianExt for DataCursorRef<'_> {
    #[inline]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }
}

impl SeekExt for DataCursorRef<'_> {
    #[inline]
    fn position(&mut self) -> usize {
        self.position
    }

    #[inline]
    fn set_position(&mut self, position: usize) -> Result<usize, DataError> {
        self.position = position.min(self.data.len());
        Ok(self.position)
    }

    #[inline]
    fn len(&mut self) -> Result<usize, DataError> {
        Ok(self.data.len())
    }
}

impl ReadExt for DataCursorRef<'_> {
    #[inline]
    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DataError> {
        ensure!(self.position.saturating_add(N) <= self.data.len(), EndOfFileSnafu);

        let mut result: MaybeUninit<[u8; N]> = MaybeUninit::uninit();
        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment. We use
        // MaybeUninit here to skip some overhead when we immediately overwrite it with new data.
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.position),
                result.as_mut_ptr().cast(),
                N,
            );
        }
        self.position = self.position.saturating_add(N);
        // SAFETY: We've initialized this with data, so it's safe.
        Ok(unsafe { result.assume_init() })
    }

    #[inline]
    fn read_length(&mut self, buffer: &mut [u8]) -> Result<usize, DataError> {
        let length = buffer.len().min(self.data.len().saturating_sub(self.position));

        // SAFETY: We're within the bounds of both `buf` and `self.data`, and will always have a valid
        // alignment. There is no way to get a mutable reference to the inner data, so buffer cannot overlap.
        unsafe {
            let src_ptr = self.data.as_ptr().add(self.position);
            core::ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), length);
        }
        self.position = self.position.saturating_add(length);
        Ok(length)
    }

    #[inline]
    #[cfg(not(feature = "alloc"))]
    fn read_slice(&mut self, length: usize) -> Result<&[u8], DataError> {
        ensure!(
            self.position.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, length)
        };
        self.position += length;
        Ok(result)
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn read_slice(&mut self, length: usize) -> Result<Cow<[u8]>, DataError> {
        ensure!(
            self.position.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, length)
        };
        self.position = self.position.saturating_add(length);
        Ok(Cow::Borrowed(result))
    }

    #[inline]
    #[cfg(not(feature = "alloc"))]
    fn remaining_slice(&mut self) -> Result<&[u8], DataError> {
        // SAFETY: We're within bounds since we're reading to the end, and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, self.data.len().saturating_sub(self.position))
        };
        self.position = self.data.len();
        Ok(result)
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn remaining_slice(&mut self) -> Result<Cow<[u8]>, DataError> {
        // SAFETY: We're within bounds since we're reading to the end, and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, self.data.len().saturating_sub(self.position))
        };
        self.position = self.data.len();
        Ok(Cow::Borrowed(result))
    }
}

impl Deref for DataCursorRef<'_> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

/// A mutable, in-memory file that allows endian-aware read and write.
///
/// This is architected to assume a fixed length, and is `no_std` compatible.
#[derive(Debug)]
pub struct DataCursorMut<'a> {
    data: &'a mut [u8],
    position: usize,
    endian: Endian,
}

impl<'a> DataCursorMut<'a> {
    /// Creates a new `DataCursorMut` with the given data and endianness.
    #[inline]
    pub fn new(data: &'a mut [u8], endian: Endian) -> Self {
        Self { data, position: 0, endian }
    }

    /// Consumes the `DataCursorMut` and returns the underlying data.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> &'a mut [u8] {
        self.data
    }

    /// Copies data from this `DataCursorMut` to another mutable slice.
    #[inline]
    pub fn copy_data_to(&self, other: &mut [u8]) {
        let len = self.data.len().min(other.len());
        // SAFETY: We're within bounds of both slices, and they don't overlap.
        unsafe {
            core::ptr::copy_nonoverlapping(self.data.as_ptr(), other.as_mut_ptr(), len);
        }
    }

    /// Copies data within the `DataCursorMut` from one range to another position.
    ///
    /// # Example
    /// ```
    /// # use orthrus_core::prelude::*;
    /// let mut data = [1, 2, 3, 4, 5];
    /// let mut cursor = DataCursorMut::new(&mut data, Endian::Little);
    /// cursor.copy_within(1..4, 2).unwrap();
    /// assert_eq!(&cursor.into_inner()[..], &[1, 2, 2, 3, 4]);
    /// ```
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if either the source range or the destination
    /// range would be out of bounds.
    #[inline]
    pub fn copy_within(&mut self, src: core::ops::Range<usize>, dest: usize) -> Result<(), DataError> {
        let length = src.end.saturating_sub(src.start);
        ensure!(
            src.end <= self.data.len() && dest.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        if src.contains(&dest) {
            for i in 0..length {
                // SAFETY: We want specific behavior if they do overlap, due to how Yaz0 compression works.
                // Both ranges are within bounds and have a valid alignment.
                unsafe {
                    *self.data.as_mut_ptr().add(dest.saturating_add(i)) =
                        *self.data.as_ptr().add(src.start.saturating_add(i));
                }
            }
        } else {
            // SAFETY: Both ranges are within bounds, do not overlap, and have a valid alignment.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.data.as_ptr().add(src.start),
                    self.data.as_mut_ptr().add(dest),
                    length,
                );
            }
        }
        Ok(())
    }
}

impl EndianExt for DataCursorMut<'_> {
    #[inline]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }
}

impl SeekExt for DataCursorMut<'_> {
    #[inline]
    fn position(&mut self) -> usize {
        self.position
    }

    #[inline]
    fn set_position(&mut self, position: usize) -> Result<usize, DataError> {
        self.position = position.min(self.data.len());
        Ok(self.position)
    }

    #[inline]
    fn len(&mut self) -> Result<usize, DataError> {
        Ok(self.data.len())
    }
}

impl ReadExt for DataCursorMut<'_> {
    #[inline]
    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DataError> {
        ensure!(self.position.saturating_add(N) <= self.data.len(), EndOfFileSnafu);

        let mut result: MaybeUninit<[u8; N]> = MaybeUninit::uninit();
        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment. We use
        // MaybeUninit here to skip some overhead when we immediately overwrite it with new data.
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.position),
                result.as_mut_ptr().cast(),
                N,
            );
        }
        self.position = self.position.saturating_add(N);
        // SAFETY: We've initialized this with data, so it's safe.
        Ok(unsafe { result.assume_init() })
    }

    #[inline]
    fn read_length(&mut self, buffer: &mut [u8]) -> Result<usize, DataError> {
        let length = buffer.len().min(self.data.len().saturating_sub(self.position));

        // SAFETY: We're within the bounds of both `buf` and `self.data`, and will always have a valid
        // alignment. There is no way to get a mutable reference to the inner data, so buffer cannot overlap.
        unsafe {
            let src_ptr = self.data.as_ptr().add(self.position);
            core::ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), length);
        }
        self.position = self.position.saturating_add(length);
        Ok(length)
    }

    #[inline]
    #[cfg(not(feature = "alloc"))]
    fn read_slice(&mut self, length: usize) -> Result<&[u8], DataError> {
        ensure!(
            self.position.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, length)
        };
        self.position += length;
        Ok(result)
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn read_slice(&mut self, length: usize) -> Result<Cow<[u8]>, DataError> {
        ensure!(
            self.position.saturating_add(length) <= self.data.len(),
            EndOfFileSnafu
        );

        // SAFETY: We're within bounds of `self.data` and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, length)
        };
        self.position = self.position.saturating_add(length);
        Ok(Cow::Borrowed(result))
    }

    #[inline]
    #[cfg(not(feature = "alloc"))]
    fn remaining_slice(&mut self) -> Result<&[u8], DataError> {
        // SAFETY: We're within bounds since we're reading to the end, and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, self.data.len().saturating_sub(self.position))
        };
        self.position = self.data.len();
        Ok(result)
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn remaining_slice(&mut self) -> Result<Cow<[u8]>, DataError> {
        // SAFETY: We're within bounds since we're reading to the end, and will always have a valid alignment.
        let result = unsafe {
            let ptr = self.data.as_ptr().add(self.position);
            core::slice::from_raw_parts(ptr, self.data.len().saturating_sub(self.position))
        };
        self.position = self.data.len();
        Ok(Cow::Borrowed(result))
    }
}

impl WriteExt for DataCursorMut<'_> {
    #[inline]
    fn write_exact<const N: usize>(&mut self, bytes: &[u8; N]) -> Result<(), DataError> {
        ensure!(self.position.saturating_add(N) <= self.data.len(), EndOfFileSnafu);

        // SAFETY: We're within the bounds of `self.data`, `bytes` will always be valid, and we'll always have
        // a valid alignment.
        unsafe {
            let dst_ptr = self.data.as_mut_ptr().add(self.position);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst_ptr, N);
        }
        self.position = self.position.saturating_add(N);
        Ok(())
    }
}

impl Deref for DataCursorMut<'_> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl DerefMut for DataCursorMut<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl AsMut<[u8]> for DataCursorMut<'_> {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        self.data
    }
}

/// A stream that allows endian-aware read and write.
///
/// This struct is generic over any type `T` that implements some combination of
/// `Read`, `Write`, and `Seek`. Methods are conditionally available based on
/// the traits implemented by `T`.
#[derive(Debug)]
pub struct DataStream<T> {
    inner: T,
    position: u64,
    endian: Endian,
}

impl<T> DataStream<T> {
    /// Creates a new `DataStream` with the given inner stream and endianness.
    #[inline]
    pub fn new(inner: T, endian: Endian) -> Self {
        Self { inner, position: 0, endian }
    }
}

impl<T> EndianExt for DataStream<T> {
    #[inline]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }
}

impl<T: Seek> SeekExt for DataStream<T> {
    #[inline]
    fn position(&mut self) -> usize {
        self.position as usize
    }

    #[inline]
    fn set_position(&mut self, position: usize) -> Result<usize, DataError> {
        self.position = self.inner.seek(SeekFrom::Start(position as u64)).context(IoSnafu)?;
        Ok(self.position as usize)
    }

    /// Returns the total length of the data.
    ///
    /// Note that this can be an expensive operation due to seeking. You should instead use something like
    /// [`std::fs::Metadata::len`].
    ///
    /// # Errors
    /// Returns an error if unable to determine the length of the stream.
    #[inline]
    fn len(&mut self) -> Result<usize, DataError> {
        let length = self.inner.seek(SeekFrom::End(0)).context(IoSnafu)?;

        // Avoid seeking a third time when we were already at the end of the
        // stream. The branch is usually way cheaper than a seek operation.
        if self.position != length {
            self.inner.seek(SeekFrom::Start(self.position)).context(IoSnafu)?;
        }

        Ok(length as usize)
    }
}

impl<T: Read> ReadExt for DataStream<T> {
    #[inline]
    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DataError> {
        let mut buffer = [0u8; N];
        self.inner.read_exact(&mut buffer).context(IoSnafu)?;
        self.position = self.position.saturating_add(N as u64);
        Ok(buffer)
    }

    #[inline]
    fn read_length(&mut self, buffer: &mut [u8]) -> Result<usize, DataError> {
        match self.inner.read_exact(buffer) {
            Ok(()) => {
                self.position = buffer.len() as u64;
                Ok(buffer.len())
            }
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                let actually_read = self.inner.read(buffer).context(IoSnafu)?;
                self.position = self.position.saturating_add(actually_read as u64);
                Ok(actually_read)
            }
            Err(e) => Err(DataError::Io { source: e }),
        }
    }

    #[inline]
    fn read_slice(&mut self, length: usize) -> Result<Cow<[u8]>, DataError> {
        let mut buffer = vec![0u8; length];
        self.inner.read_exact(&mut buffer).context(IoSnafu)?;
        self.position = self.position.saturating_add(length as u64);
        Ok(Cow::Owned(buffer))
    }

    #[inline]
    fn remaining_slice(&mut self) -> Result<Cow<[u8]>, DataError> {
        let mut buffer = Vec::new();
        self.inner.read_to_end(&mut buffer).context(IoSnafu)?;
        self.position = self.position.saturating_add(buffer.len() as u64);
        Ok(Cow::Owned(buffer))
    }
}

impl<T: Write> WriteExt for DataStream<T> {
    #[inline]
    fn write_exact<const N: usize>(&mut self, bytes: &[u8; N]) -> Result<(), DataError> {
        self.inner.write_all(bytes).context(IoSnafu)?;
        self.position = self.position.saturating_add(N as u64);
        Ok(())
    }
}

impl<T> Deref for DataStream<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for DataStream<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
