//! Endian-aware data manipulation for efficient byte slice operations.
//!
//! This crate contains several types that are meant to wrap a byte slice and provide a convenient
//! interface for reading and writing primitive data types from it.
//! * [`DataCursor`] is the owned variant, where it owns the byte slice directly, for use as an
//!   in-memory file.
//! * [`DataCursorRef`] is the borrowed immutable variant, that wraps a reference to a byte slice
//!   and provides reading.
//! * [`DataCursorMut`] is the borrowed mutable variant, that wraps a reference to a byte slice and
//!   provides reading and writing.
//!
//! These cursors work similarly to the [`std::io`] module, wherein you have to include specific
//! traits for functionality.
//! * [`DataCursorTrait`] provides the basic methods for using a cursor, and allows for trait
//!   bounds.
//! * [`EndianRead`] provides reading Rust primitives using the stored endianness.
//! * [`EndianWrite`] provides writing Rust primitives using the stored endianness.

//This whole module is held up on safe transmute
use core::ops::{Deref, DerefMut};
#[cfg(feature = "std")]
use std::{io::prelude::*, path::Path};

use snafu::prelude::*;

#[cfg(not(feature = "std"))]
use crate::no_std::*;

/// Error conditions for when reading/writing data.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    /// Thrown if reading/writing tries to go out of bounds.
    #[snafu(display("Unexpected End-Of-File!"))]
    EndOfFile,
    /// Thrown if trying to resize a cursor to larger than the current size.
    #[snafu(display("Invalid End Size!"))]
    InvalidSize,
    /// Thrown if UTF-8 validation fails when converting a string.
    #[snafu(display("Invalid UTF-8 String!"))]
    InvalidUtf8,
}
type Result<T> = core::result::Result<T, Error>;

/// Allows specifying which endianness the cursor is currently working with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

impl Default for Endian {
    #[cfg(target_endian = "little")]
    #[inline]
    fn default() -> Self {
        Self::Little
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn default() -> Self {
        Self::Big
    }
}

pub enum SeekFrom {
    Start(usize),
    End(isize),
    Current(isize),
}

/// Shared cursor functionality, e.g. utilities
pub trait DataCursorTrait {
    /// Returns the current position of this cursor.
    fn position(&self) -> usize;

    /// Sets the position of this cursor.
    fn set_position(&mut self, pos: usize);

    /// Sets the position of this cursor.
    fn seek(&mut self, style: SeekFrom) -> Result<usize>;

    /// Returns the current endianness of this cursor.
    fn endian(&self) -> Endian;

    /// Sets the endianness of this cursor.
    fn set_endian(&mut self, endian: Endian);

    /// Returns the remaining data from the current position.
    fn remaining_slice(&self) -> &[u8];

    /// Returns `true` if the remaining slice is empty.
    fn is_empty(&self) -> bool;

    /// Returns the length of the currently stored data.
    fn len(&self) -> usize;

    /// Returns a slice from the current position to some additional length.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn get_slice(&mut self, length: usize) -> Result<&[u8]>;

    /// Attempts to fill the buffer with data. Mainly intended for `no_std`, where the [`Read`]
    /// trait is not available.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_length(&mut self, buf: &mut [u8]) -> Result<()>;
}

macro_rules! datacursor_read {
    ($self:ident, $t:ty) => {{
        const LENGTH: usize = core::mem::size_of::<$t>();
        // Bounds check to ensure we're within the valid data range
        ensure!($self.len() >= $self.pos + LENGTH, EndOfFileSnafu);

        unsafe {
            // SAFETY: pointer::add should always be safe if we have a valid box/slice
            let ptr: *const $t = $self.data.as_ptr().add($self.pos).cast();
            $self.pos += LENGTH;

            // SAFETY: We can't guarantee that the pointer is aligned, so we use read_unaligned
            match $self.endian {
                Endian::Little => Ok(<$t>::from_le(ptr.read_unaligned())),
                Endian::Big => Ok(<$t>::from_be(ptr.read_unaligned())),
            }
        }
    }};
}

/// Endian-aware reading of Rust primitives
pub trait EndianRead {
    /// Reads one byte and return it as a `u8`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_u8(&mut self) -> Result<u8>;

    /// Reads one byte and return it as an `i8`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_i8(&mut self) -> Result<i8>;

    /// Reads two bytes and return it as a `u16`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_u16(&mut self) -> Result<u16>;

    /// Reads two bytes and return it as an `i16`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_i16(&mut self) -> Result<i16>;

    /// Reads four bytes and return it as a `u32`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_u32(&mut self) -> Result<u32>;

    /// Reads four bytes and return it as an `i32`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_i32(&mut self) -> Result<i32>;

    /// Reads eight bytes and return it as a `u64`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_u64(&mut self) -> Result<u64>;

    /// Reads eight bytes and return it as an `i64`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_i64(&mut self) -> Result<i64>;

    /// Reads sixteen bytes and return it as a `u128`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_u128(&mut self) -> Result<u128>;

    /// Reads sixteen bytes and return it as an `i128`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_i128(&mut self) -> Result<i128>;

    /// Reads four bytes and return it as an `f32`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_f32(&mut self) -> Result<f32>;

    /// Reads eight bytes and return it as an `f64`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read out of bounds.
    fn read_f64(&mut self) -> Result<f64>;
}

macro_rules! datacursor_write {
    ($self:ident, $value:expr, $t:ty) => {{
        const LENGTH: usize = core::mem::size_of::<$t>();
        // Bounds check to ensure we're within the valid data range
        ensure!($self.len() >= $self.pos + LENGTH, EndOfFileSnafu);

        unsafe {
            // SAFETY: pointer::add should always be safe if we have a valid box/slice
            let ptr: *mut $t = $self.data.as_mut_ptr().add($self.pos).cast();
            $self.pos += LENGTH;

            // SAFETY: We can't guarantee that the pointer is aligned, so we use write_unaligned
            match $self.endian {
                Endian::Little => ptr.write_unaligned($value.to_le()),
                Endian::Big => ptr.write_unaligned($value.to_be()),
            }
        }
        Ok(())
    }};
}

/// Endian-aware writing of Rust primitives
pub trait EndianWrite {
    /// Writes one byte from a `u8`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_u8(&mut self, value: u8) -> Result<()>;

    /// Writes one byte from an `i8`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_i8(&mut self, value: i8) -> Result<()>;

    /// Writes two bytes from a `u16`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_u16(&mut self, value: u16) -> Result<()>;

    /// Writes two bytes from an `i16`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_i16(&mut self, value: i16) -> Result<()>;

    /// Writes four bytes from a `u32`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_u32(&mut self, value: u32) -> Result<()>;

    /// Writes four bytes from an `i32`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_i32(&mut self, value: i32) -> Result<()>;

    /// Writes eight bytes from a `u64`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_u64(&mut self, value: u64) -> Result<()>;

    /// Writes eight bytes from an `i64`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_i64(&mut self, value: i64) -> Result<()>;

    /// Writes sixteen bytes from a `u128`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_u128(&mut self, value: u128) -> Result<()>;

    /// Writes sixteen bytes from an `i128`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_i128(&mut self, value: i128) -> Result<()>;

    /// Writes four bytes from an `f32`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_f32(&mut self, value: f32) -> Result<()>;

    /// Writes eight bytes from an `f64`.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to write out of bounds.
    fn write_f64(&mut self, value: f64) -> Result<()>;
}

/// An owned, in-memory file that allows endian-aware read and write.
///
/// This is architected to assume a fixed length, which should work for the majority of use cases,
/// as users should be minimizing allocations at all costs.
#[derive(Debug, Default)]
pub struct DataCursor {
    data: Box<[u8]>,
    pos: usize,
    endian: Endian,
}

impl DataCursor {
    /// Creates a new cursor using the provided data and endianness.
    #[inline]
    pub fn new<I: Into<Box<[u8]>>>(data: I, endian: Endian) -> Self {
        Self { data: data.into(), pos: 0, endian }
    }

    /// Creates a new cursor using the provided path and endianness.
    ///
    /// # Errors
    /// This function will return an error if `path` does not exist, if unable to read file
    /// metadata, or if reading the file fails.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_path<P: AsRef<Path>>(path: P, endian: Endian) -> std::io::Result<Self> {
        Ok(Self::new(std::fs::read(path)?, endian))
    }

    /// Consumes this cursor, returning the underlying data.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> Box<[u8]> {
        self.data
    }

    /// Tries to resize the cursor to a shorter length, consuming it and returning a new one.
    ///
    /// # Errors
    /// Returns [`InvalidSize`](Error::InvalidSize) if not actually shrinking the length.
    #[inline]
    pub fn shrink_to(self, len: usize) -> Result<Self> {
        //Make sure the new size is actually smaller. Length is unsigned, so it can't be negative
        ensure!(len < self.len(), InvalidSizeSnafu);

        let mut v = self.data.into_vec();
        v.truncate(len);
        Ok(Self::new(v, self.endian))
    }

    /// Reads a byte from this cursor and writes it to the output.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read or write out of bounds.
    #[inline]
    pub fn copy_byte_to(&mut self, output: &mut Self) -> Result<()> {
        const LENGTH: usize = core::mem::size_of::<u8>();
        // Bounds check to ensure we're within the valid data range
        ensure!(
            (output.len() >= output.pos + LENGTH) && (self.len() >= self.pos + LENGTH),
            EndOfFileSnafu
        );

        // SAFETY: Box ensures that the pointer arithmetic here is safe
        unsafe { *output.data.as_mut_ptr().add(output.pos) = *self.data.as_ptr().add(self.pos) };
        self.pos += LENGTH;
        output.pos += LENGTH;
        Ok(())
    }

    /// Copies a number of bytes from elsewhere in the cursor to the current position.
    ///
    /// `src` is an offset from the start of the cursor, `length` is the number of bytes to copy.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read or write out of bounds.
    #[inline]
    pub fn copy_within(&mut self, src: usize, length: usize) -> Result<()> {
        // Bounds check to ensure both the source slice and current slice are within data bounds.
        ensure!(
            (self.len() >= src + length) && (self.len() >= self.pos + length),
            EndOfFileSnafu
        );

        // Check if the ranges overlap, otherwise use the faster copy method
        if (src <= self.pos + length) && (self.pos <= src + length) {
            for n in 0..length {
                unsafe {
                    *self.data.as_mut_ptr().add(self.pos + n) = *self.data.as_ptr().add(src + n);
                }
            }
        } else {
            unsafe {
                let src_ptr = self.data.as_ptr().add(src);
                let dest_ptr = self.data.as_mut_ptr().add(self.pos);
                core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, length);
            }
        }

        self.pos += length;
        Ok(())
    }
}

impl DataCursorTrait for DataCursor {
    #[inline]
    #[must_use]
    fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    #[inline]
    fn seek(&mut self, style: SeekFrom) -> Result<usize> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.data.len(), n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        match base_pos.checked_add_signed(offset) {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => EndOfFileSnafu.fail()?,
        }
    }

    #[inline]
    #[must_use]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    #[inline]
    #[must_use]
    fn remaining_slice(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    #[inline]
    #[must_use]
    fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    #[inline]
    #[must_use]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn get_slice(&mut self, length: usize) -> Result<&[u8]> {
        ensure!(self.len() >= self.pos + length, EndOfFileSnafu);
        let slice = &self.data[self.pos..self.pos + length];
        self.pos += length;
        Ok(slice)
    }

    #[inline]
    fn read_length(&mut self, buf: &mut [u8]) -> Result<()> {
        ensure!(buf.len() <= self.data.len() - self.pos, EndOfFileSnafu);
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

impl EndianRead for DataCursor {
    #[inline]
    fn read_u8(&mut self) -> Result<u8> {
        const LENGTH: usize = core::mem::size_of::<u8>();
        // Bounds check to ensure we're within the valid data range
        ensure!(self.len() >= self.pos + LENGTH, EndOfFileSnafu);

        // SAFETY: Box ensures that the pointer arithmetic here is safe
        let value = unsafe { *self.data.as_ptr().add(self.pos) };
        self.pos += LENGTH;
        Ok(value)
    }

    #[inline]
    fn read_i8(&mut self) -> Result<i8> {
        self.read_u8().map(|v| v as i8)
    }

    #[inline]
    fn read_u16(&mut self) -> Result<u16> {
        datacursor_read!(self, u16)
    }

    #[inline]
    fn read_i16(&mut self) -> Result<i16> {
        datacursor_read!(self, i16)
    }

    #[inline]
    fn read_u32(&mut self) -> Result<u32> {
        datacursor_read!(self, u32)
    }

    #[inline]
    fn read_i32(&mut self) -> Result<i32> {
        datacursor_read!(self, i32)
    }

    #[inline]
    fn read_u64(&mut self) -> Result<u64> {
        datacursor_read!(self, u64)
    }

    #[inline]
    fn read_i64(&mut self) -> Result<i64> {
        datacursor_read!(self, i64)
    }

    #[inline]
    fn read_u128(&mut self) -> Result<u128> {
        datacursor_read!(self, u128)
    }

    #[inline]
    fn read_i128(&mut self) -> Result<i128> {
        datacursor_read!(self, i128)
    }

    #[inline]
    fn read_f32(&mut self) -> Result<f32> {
        datacursor_read!(self, u32).map(f32::from_bits)
    }

    #[inline]
    fn read_f64(&mut self) -> Result<f64> {
        datacursor_read!(self, u64).map(f64::from_bits)
    }
}

impl EndianWrite for DataCursor {
    #[inline]
    fn write_u8(&mut self, value: u8) -> Result<()> {
        const LENGTH: usize = core::mem::size_of::<u8>();
        // Bounds check to ensure we're within the valid data range
        ensure!(self.len() >= self.pos + LENGTH, EndOfFileSnafu);

        // SAFETY: Box ensures that the pointer arithmetic here is safe
        unsafe {
            *self.data.as_mut_ptr().add(self.pos) = value;
        }
        self.pos += LENGTH;
        Ok(())
    }

    #[inline]
    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.write_u8(value as u8)
    }

    #[inline]
    fn write_u16(&mut self, value: u16) -> Result<()> {
        datacursor_write!(self, value, u16)
    }

    #[inline]
    fn write_i16(&mut self, value: i16) -> Result<()> {
        datacursor_write!(self, value, i16)
    }

    #[inline]
    fn write_u32(&mut self, value: u32) -> Result<()> {
        datacursor_write!(self, value, u32)
    }

    #[inline]
    fn write_i32(&mut self, value: i32) -> Result<()> {
        datacursor_write!(self, value, i32)
    }

    #[inline]
    fn write_u64(&mut self, value: u64) -> Result<()> {
        datacursor_write!(self, value, u64)
    }

    #[inline]
    fn write_i64(&mut self, value: i64) -> Result<()> {
        datacursor_write!(self, value, i64)
    }

    #[inline]
    fn write_u128(&mut self, value: u128) -> Result<()> {
        datacursor_write!(self, value, u128)
    }

    #[inline]
    fn write_i128(&mut self, value: i128) -> Result<()> {
        datacursor_write!(self, value, i128)
    }

    #[inline]
    fn write_f32(&mut self, value: f32) -> Result<()> {
        datacursor_write!(self, value.to_bits(), u32)
    }

    #[inline]
    fn write_f64(&mut self, value: f64) -> Result<()> {
        datacursor_write!(self, value.to_bits(), u64)
    }
}

#[cfg(feature = "std")]
impl Read for DataCursor {
    /// Fills `buf` either fully or until end-of-file.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let length = core::cmp::min(buf.len(), self.len() - self.pos);

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                length,
            );
        }
        self.pos += length;
        Ok(length)
    }

    /// Reads bytes until end-of-file and puts them in `buf`.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let length = self.len() - self.pos;
        buf.extend_from_slice(&self.data[self.pos..]);
        self.pos = self.len();
        Ok(length)
    }

    /// Attempts to fill the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) if there
    /// isn't enough data to fill `buf`.
    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if buf.len() > self.len() - self.pos {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

#[cfg(feature = "std")]
impl Write for DataCursor {
    /// Fills `buf` either fully, or until end-of-file.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let length = core::cmp::min(buf.len(), self.len() - self.pos);

        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.data.as_mut_ptr().add(self.pos),
                length,
            );
        }
        self.pos += length;
        Ok(length)
    }

    /// Cursor is held entirely in memory, so `flush` is a no-op.
    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// Attempts to fill the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`WriteZero`](std::io::ErrorKind::WriteZero) if there
    /// isn't enough data to fill `buf`.
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if self.pos + buf.len() > self.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::WriteZero));
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.data.as_mut_ptr().add(self.pos),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

#[cfg(feature = "std")]
impl BufRead for DataCursor {
    #[inline]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(self.remaining_slice())
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }
}

impl From<Box<[u8]>> for DataCursor {
    #[inline]
    fn from(value: Box<[u8]>) -> Self {
        Self { data: value, pos: 0, endian: Endian::default() }
    }
}

#[cfg(feature = "std")]
impl From<Vec<u8>> for DataCursor {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self {
            data: value.into_boxed_slice(),
            pos: 0,
            endian: Endian::default(),
        }
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

/// An immutably borrowed, in-memory file that allows endian-aware read.
///
/// This is architected to assume a fixed length, which should work for the majority of use cases,
/// as users should be minimizing allocations at all costs.
#[derive(Debug, Default)]
pub struct DataCursorRef<'a> {
    data: &'a [u8],
    pos: usize,
    endian: Endian,
}

impl<'a> DataCursorRef<'a> {
    /// Creates a new cursor using the provided data and endianness.
    #[inline]
    #[must_use]
    pub fn new(data: &'a [u8], endian: Endian) -> Self {
        Self { data, pos: 0, endian }
    }

    /// Consumes this cursor, returning  the underlying data.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> &'a [u8] {
        self.data
    }

    /// Tries to resize the cursor to a shorter length, consuming it and returning a new one.
    ///
    /// # Errors
    /// Returns [`InvalidSize`](Error::InvalidSize) if not actually shrinking the length.
    #[inline]
    pub fn shrink_to(self, len: usize) -> Result<Self> {
        //Make sure the new size is actually smaller
        ensure!(len < self.len(), InvalidSizeSnafu);
        Ok(Self::new(&self.data[..len], self.endian))
    }
}

impl DataCursorTrait for DataCursorRef<'_> {
    #[inline]
    #[must_use]
    fn position(&self) -> usize {
        self.pos
    }

    /// Sets the position of this cursor.
    #[inline]
    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    #[inline]
    fn seek(&mut self, style: SeekFrom) -> Result<usize> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.data.len(), n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        match base_pos.checked_add_signed(offset) {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => EndOfFileSnafu.fail()?,
        }
    }

    #[inline]
    #[must_use]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    #[inline]
    #[must_use]
    fn remaining_slice(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    #[inline]
    #[must_use]
    fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    #[inline]
    #[must_use]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn get_slice(&mut self, length: usize) -> Result<&[u8]> {
        ensure!(self.len() >= self.pos + length, EndOfFileSnafu);
        let slice = &self.data[self.pos..self.pos + length];
        self.pos += length;
        Ok(slice)
    }

    #[inline]
    fn read_length(&mut self, buf: &mut [u8]) -> Result<()> {
        ensure!(buf.len() <= self.data.len() - self.pos, EndOfFileSnafu);
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

impl EndianRead for DataCursorRef<'_> {
    #[inline]
    fn read_u8(&mut self) -> Result<u8> {
        const LENGTH: usize = core::mem::size_of::<u8>();
        // Bounds check to ensure we're within the valid data range
        ensure!(self.len() >= self.pos + LENGTH, EndOfFileSnafu);

        // SAFETY: u8 slices should always be valid
        let value = unsafe { *self.data.as_ptr().add(self.pos) };
        self.pos += LENGTH;
        Ok(value)
    }

    #[inline]
    fn read_i8(&mut self) -> Result<i8> {
        self.read_u8().map(|v| v as i8)
    }

    #[inline]
    fn read_u16(&mut self) -> Result<u16> {
        datacursor_read!(self, u16)
    }

    #[inline]
    fn read_i16(&mut self) -> Result<i16> {
        datacursor_read!(self, i16)
    }

    #[inline]
    fn read_u32(&mut self) -> Result<u32> {
        datacursor_read!(self, u32)
    }

    #[inline]
    fn read_i32(&mut self) -> Result<i32> {
        datacursor_read!(self, i32)
    }

    #[inline]
    fn read_u64(&mut self) -> Result<u64> {
        datacursor_read!(self, u64)
    }

    #[inline]
    fn read_i64(&mut self) -> Result<i64> {
        datacursor_read!(self, i64)
    }

    #[inline]
    fn read_u128(&mut self) -> Result<u128> {
        datacursor_read!(self, u128)
    }

    #[inline]
    fn read_i128(&mut self) -> Result<i128> {
        datacursor_read!(self, i128)
    }

    #[inline]
    fn read_f32(&mut self) -> Result<f32> {
        datacursor_read!(self, u32).map(f32::from_bits)
    }

    #[inline]
    fn read_f64(&mut self) -> Result<f64> {
        datacursor_read!(self, u64).map(f64::from_bits)
    }
}

#[cfg(feature = "std")]
impl Read for DataCursorRef<'_> {
    /// Fills `buf` either fully or until end-of-file.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let length = core::cmp::min(buf.len(), self.len() - self.pos);

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                length,
            );
        }
        self.pos += length;
        Ok(length)
    }

    /// Reads bytes until end-of-file and puts them in `buf`.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let length = self.len() - self.pos;
        buf.extend_from_slice(&self.data[self.pos..]);
        self.pos = self.len();
        Ok(length)
    }

    /// Attempts to fill the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) if there
    /// isn't enough data to fill `buf`.
    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if buf.len() > self.len() - self.pos {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

#[cfg(feature = "std")]
impl BufRead for DataCursorRef<'_> {
    #[inline]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(self.remaining_slice())
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }
}

impl Deref for DataCursorRef<'_> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

/// A mutably borrowed, in-memory file that allows endian-aware read and write.
///
/// This is architected to assume a fixed length, which should work for the majority of use cases,
/// as users should be minimizing allocations at all costs.
#[derive(Debug, Default)]
pub struct DataCursorMut<'a> {
    data: &'a mut [u8],
    pos: usize,
    endian: Endian,
}

impl<'a> DataCursorMut<'a> {
    /// Creates a new cursor using the provided data and endianness.
    #[inline]
    pub fn new(data: &'a mut [u8], endian: Endian) -> Self {
        Self { data, pos: 0, endian }
    }

    /// Consumes this cursor, returning  the underlying data.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> &'a mut [u8] {
        self.data
    }

    /// Tries to resize the cursor to a shorter length, consuming it and returning a new one.
    ///
    /// # Errors
    /// Returns [`InvalidSize`](Error::InvalidSize) if not actually shrinking the length.
    #[inline]
    pub fn shrink_to(self, len: usize) -> Result<Self> {
        //Make sure the new size is actually smaller
        ensure!(len < self.len(), InvalidSizeSnafu);

        Ok(Self::new(&mut self.data[..len], self.endian))
    }

    /// Copies a byte from the cursor to the output.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read or write out of bounds.
    #[inline]
    pub fn copy_byte_to(&mut self, output: &mut Self) -> Result<()> {
        const LENGTH: usize = core::mem::size_of::<u8>();
        // Bounds check to ensure we're within the valid data range
        ensure!(
            (output.len() >= output.pos + LENGTH) && (self.len() >= self.pos + LENGTH),
            EndOfFileSnafu
        );

        // SAFETY: u8 slices should always be valid
        unsafe { *output.data.as_mut_ptr().add(output.pos) = *self.data.as_ptr().add(self.pos) };
        self.pos += LENGTH;
        output.pos += LENGTH;
        Ok(())
    }

    /// Copies a number of bytes from elsewhere in the cursor to the current position.
    ///
    /// `src` is an offset from the start of the cursor, `length` is the number of bytes to copy.
    ///
    /// # Errors
    /// Returns [`EndOfFile`](Error::EndOfFile) if trying to read or write out of bounds.
    #[inline]
    pub fn copy_within(&mut self, src: usize, length: usize) -> Result<()> {
        // Bounds check to ensure both the source slice and current slice are within data bounds.
        ensure!(
            (self.len() >= src + length) && (self.len() >= self.pos + length),
            EndOfFileSnafu
        );

        // Check if the ranges overlap, otherwise use the faster copy method
        if (src <= self.pos + length) && (self.pos <= src + length) {
            for n in 0..length {
                unsafe {
                    *self.data.as_mut_ptr().add(self.pos + n) = *self.data.as_ptr().add(src + n);
                }
            }
        } else {
            unsafe {
                let src_ptr = self.data.as_ptr().add(src);
                let dest_ptr = self.data.as_mut_ptr().add(self.pos);
                core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, length);
            }
        }

        self.pos += length;
        Ok(())
    }
}

impl DataCursorTrait for DataCursorMut<'_> {
    #[inline]
    #[must_use]
    fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    #[inline]
    fn seek(&mut self, style: SeekFrom) -> Result<usize> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.data.len(), n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        match base_pos.checked_add_signed(offset) {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => EndOfFileSnafu.fail()?,
        }
    }

    #[inline]
    #[must_use]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    #[inline]
    #[must_use]
    fn remaining_slice(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    #[inline]
    #[must_use]
    fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    #[inline]
    #[must_use]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn get_slice(&mut self, length: usize) -> Result<&[u8]> {
        ensure!(self.len() >= self.pos + length, EndOfFileSnafu);
        let slice = &self.data[self.pos..self.pos + length];
        self.pos += length;
        Ok(slice)
    }

    #[inline]
    fn read_length(&mut self, buf: &mut [u8]) -> Result<()> {
        ensure!(buf.len() <= self.data.len() - self.pos, EndOfFileSnafu);
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

impl EndianRead for DataCursorMut<'_> {
    #[inline]
    fn read_u8(&mut self) -> Result<u8> {
        const LENGTH: usize = core::mem::size_of::<u8>();
        // Bounds check to ensure we're within the valid data range
        ensure!(self.len() >= self.pos + LENGTH, EndOfFileSnafu);

        // SAFETY: u8 slices should always be valid
        let value = unsafe { *self.data.as_ptr().add(self.pos) };
        self.pos += LENGTH;
        Ok(value)
    }

    #[inline]
    fn read_i8(&mut self) -> Result<i8> {
        self.read_u8().map(|v| v as i8)
    }

    #[inline]
    fn read_u16(&mut self) -> Result<u16> {
        datacursor_read!(self, u16)
    }

    #[inline]
    fn read_i16(&mut self) -> Result<i16> {
        datacursor_read!(self, i16)
    }

    #[inline]
    fn read_u32(&mut self) -> Result<u32> {
        datacursor_read!(self, u32)
    }

    #[inline]
    fn read_i32(&mut self) -> Result<i32> {
        datacursor_read!(self, i32)
    }

    #[inline]
    fn read_u64(&mut self) -> Result<u64> {
        datacursor_read!(self, u64)
    }

    #[inline]
    fn read_i64(&mut self) -> Result<i64> {
        datacursor_read!(self, i64)
    }

    #[inline]
    fn read_u128(&mut self) -> Result<u128> {
        datacursor_read!(self, u128)
    }

    #[inline]
    fn read_i128(&mut self) -> Result<i128> {
        datacursor_read!(self, i128)
    }

    #[inline]
    fn read_f32(&mut self) -> Result<f32> {
        datacursor_read!(self, u32).map(f32::from_bits)
    }

    #[inline]
    fn read_f64(&mut self) -> Result<f64> {
        datacursor_read!(self, u64).map(f64::from_bits)
    }
}

impl EndianWrite for DataCursorMut<'_> {
    #[inline]
    fn write_u8(&mut self, value: u8) -> Result<()> {
        const LENGTH: usize = core::mem::size_of::<u8>();
        // Bounds check to ensure we're within the valid data range
        ensure!(self.len() >= self.pos + LENGTH, EndOfFileSnafu);

        // SAFETY: u8 slices should always be valid
        unsafe {
            *self.data.as_mut_ptr().add(self.pos) = value;
        }
        self.pos += LENGTH;
        Ok(())
    }

    #[inline]
    fn write_i8(&mut self, value: i8) -> Result<()> {
        self.write_u8(value as u8)
    }

    #[inline]
    fn write_u16(&mut self, value: u16) -> Result<()> {
        datacursor_write!(self, value, u16)
    }

    #[inline]
    fn write_i16(&mut self, value: i16) -> Result<()> {
        datacursor_write!(self, value, i16)
    }

    #[inline]
    fn write_u32(&mut self, value: u32) -> Result<()> {
        datacursor_write!(self, value, u32)
    }

    #[inline]
    fn write_i32(&mut self, value: i32) -> Result<()> {
        datacursor_write!(self, value, i32)
    }

    #[inline]
    fn write_u64(&mut self, value: u64) -> Result<()> {
        datacursor_write!(self, value, u64)
    }

    #[inline]
    fn write_i64(&mut self, value: i64) -> Result<()> {
        datacursor_write!(self, value, i64)
    }

    #[inline]
    fn write_u128(&mut self, value: u128) -> Result<()> {
        datacursor_write!(self, value, u128)
    }

    #[inline]
    fn write_i128(&mut self, value: i128) -> Result<()> {
        datacursor_write!(self, value, i128)
    }

    #[inline]
    fn write_f32(&mut self, value: f32) -> Result<()> {
        datacursor_write!(self, value.to_bits(), u32)
    }

    #[inline]
    fn write_f64(&mut self, value: f64) -> Result<()> {
        datacursor_write!(self, value.to_bits(), u64)
    }
}

#[cfg(feature = "std")]
impl Read for DataCursorMut<'_> {
    /// Fills `buf` either fully or until end-of-file.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let length = core::cmp::min(buf.len(), self.len() - self.pos);

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                length,
            );
        }
        self.pos += length;
        Ok(length)
    }

    /// Reads bytes until end-of-file and puts them in `buf`.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let length = self.len() - self.pos;
        buf.extend_from_slice(&self.data[self.pos..]);
        self.pos = self.len();
        Ok(length)
    }

    /// Attempts to fill the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) if there
    /// isn't enough data to fill `buf`.
    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if buf.len() > self.len() - self.pos {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_ptr().add(self.pos),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

#[cfg(feature = "std")]
impl Write for DataCursorMut<'_> {
    /// Fills `buf` either fully, or until end-of-file.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let length = core::cmp::min(buf.len(), self.len() - self.pos);

        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.data.as_mut_ptr().add(self.pos),
                length,
            );
        }
        self.pos += length;
        Ok(length)
    }

    /// Cursor is held entirely in memory, so `flush` is a no-op.
    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// Attempts to fill the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`WriteZero`](std::io::ErrorKind::WriteZero) if there
    /// isn't enough data to fill `buf`.
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if self.pos + buf.len() > self.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::WriteZero));
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.data.as_mut_ptr().add(self.pos),
                buf.len(),
            );
        }
        self.pos += buf.len();
        Ok(())
    }
}

#[cfg(feature = "std")]
impl BufRead for DataCursorMut<'_> {
    #[inline]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(self.remaining_slice())
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.pos += amt;
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
