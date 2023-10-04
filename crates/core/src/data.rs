use core::mem::size_of;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataCursorError {
    #[error("Unexpected End-Of-File")]
    EndOfFile,
}

#[cfg(target_endian = "little")]
#[derive(Clone, Copy, Debug, Default)]
pub enum Endian {
    #[default]
    Little,
    Big,
}

#[cfg(target_endian = "big")]
#[derive(Clone, Copy, Debug, Default)]
pub enum Endian {
    Little,
    #[default]
    Big,
}

#[derive(Debug, Default)]
pub struct DataCursor {
    data: Box<[u8]>,
    pos: usize,
    endian: Endian,
}

impl DataCursor {
    /// Creates a new `DataCursor` using the provided data and endianness.
    pub fn new<I: Into<Self>>(data: I, endian: Endian) -> Self {
        let mut cursor = data.into();
        cursor.endian = endian;
        cursor
    }

    /// Creates a new `DataCursor` using the provided path and endianness.
    ///
    /// # Errors
    /// This function will return an error if `path` does not exist, the user lacks permissions
    /// to read the file, or reading the file fails.
    pub fn from_path<P: AsRef<Path>>(path: P, endian: Endian) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let size = usize::try_from(file.metadata()?.len()).unwrap();
        let mut bytes = vec![0u8; size];
        file.read_exact(&mut bytes)?;
        Ok(Self::new(bytes, endian))
    }

    /// Sets the endian type used for multi-byte reading and writing.
    #[inline]
    pub fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    /// Returns the current endian type used for multi-byte reading and writing.
    #[inline]
    #[must_use]
    pub const fn endian(&self) -> Endian {
        self.endian
    }

    /// Sets the position of this `DataCursor`.
    #[inline]
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Returns the current position of this `DataCursor`.
    #[inline]
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Returns the length of the currently stored data.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if there is no currently stored data.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the remaining data from the current position.
    #[inline]
    #[must_use]
    pub fn remaining_slice(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    /// Reads a byte from this `DataCursor` and writes it to the other `DataCursor`.
    #[inline]
    pub fn copy_byte_to(&mut self, output: &mut Self) -> Result<(), DataCursorError> {
        output.write_u8(self.read_u8()?)?;
        Ok(())
    }

    /// Copies a number of bytes from elsewhere in the buffer to the current position.
    #[inline]
    pub fn copy_range_within(&mut self, src: usize, length: usize) -> Result<(), DataCursorError> {
        // SAFETY: Ensure the source and destination are within data bounds.
        if src + length <= self.data.len() && self.pos + length <= self.data.len() {
            // Check that the ranges don't overlap, otherwise manually copy
            if src + length < self.pos && self.pos + length < src {
                for n in 0..length {
                    unsafe {
                        *self.data.as_mut_ptr().add(src + n) =
                            *self.data.as_ptr().add(self.pos + n);
                    }
                }
            } else {
                // SAFETY: slices are valid and the same length, along with not overlapping
                unsafe {
                    let src_ptr = self.data.as_ptr().add(src);
                    let dest_ptr = self.data.as_mut_ptr().add(self.pos);
                    core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, length);
                }
            }
            self.pos += length;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Read one byte from `DataCursor` and return it as a `u8`.
    #[inline]
    pub fn read_u8(&mut self) -> Result<u8, DataCursorError> {
        const LENGTH: usize = size_of::<u8>();
        if self.pos + LENGTH <= self.data.len() {
            let value = unsafe { *self.data.as_ptr().add(self.pos) };
            self.pos += LENGTH;
            Ok(value)
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Write one byte from a `u8` into `DataCursor`.
    #[inline]
    pub fn write_u8(&mut self, value: u8) -> Result<(), DataCursorError> {
        const LENGTH: usize = size_of::<u8>();
        if self.pos + LENGTH <= self.data.len() {
            unsafe {
                *self.data.as_mut_ptr().add(self.pos) = value;
            }
            self.pos += LENGTH;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Read one byte from `DataCursor` and return it as a `i8`.
    #[inline]
    pub fn read_i8(&mut self) -> Result<i8, DataCursorError> {
        self.read_u8().map(|v| v as i8)
    }

    /// Write one byte from a `i8` into `DataCursor`.
    #[inline]
    pub fn write_i8(&mut self, value: i8) -> Result<(), DataCursorError> {
        self.write_u8(value as u8)
    }

    /// Read two bytes from `DataCursor` and return it as a `u16`.
    #[inline]
    pub fn read_u16(&mut self) -> Result<u16, DataCursorError> {
        const LENGTH: usize = size_of::<u16>();
        if self.pos + LENGTH <= self.data.len() {
            let value = unsafe {
                let ptr = self.data.as_ptr().add(self.pos).cast::<[u8; LENGTH]>();
                core::ptr::read(ptr)
            };
            self.pos += LENGTH;

            match self.endian {
                Endian::Little => Ok(u16::from_le_bytes(value)),
                Endian::Big => Ok(u16::from_be_bytes(value)),
            }
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Write two bytes from a `u16` into `DataCursor`.
    #[inline]
    pub fn write_u16(&mut self, value: u16) -> Result<(), DataCursorError> {
        const LENGTH: usize = size_of::<u16>();
        if self.pos + LENGTH <= self.data.len() {
            let bytes = match self.endian {
                Endian::Little => value.to_le_bytes(),
                Endian::Big => value.to_be_bytes(),
            };
            unsafe {
                let ptr = self.data.as_mut_ptr().add(self.pos);
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), LENGTH);
            }
            self.pos += LENGTH;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Read two bytes from `DataCursor` and return it as a `i16`.
    #[inline]
    pub fn read_i16(&mut self) -> Result<i16, DataCursorError> {
        self.read_u16().map(|v| v as i16)
    }

    /// Write two bytes from a `i16` into `DataCursor`.
    #[inline]
    pub fn write_i16(&mut self, value: i16) -> Result<(), DataCursorError> {
        self.write_u16(value as u16)
    }

    /// Read four bytes from `DataCursor` and return it as a `u32`.
    #[inline]
    pub fn read_u32(&mut self) -> Result<u32, DataCursorError> {
        const LENGTH: usize = size_of::<u32>();
        if self.pos + LENGTH <= self.data.len() {
            let value = unsafe {
                let ptr = self.data.as_ptr().add(self.pos).cast::<[u8; LENGTH]>();
                core::ptr::read(ptr)
            };
            self.pos += LENGTH;

            match self.endian {
                Endian::Little => Ok(u32::from_le_bytes(value)),
                Endian::Big => Ok(u32::from_be_bytes(value)),
            }
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Write four bytes from a `u32` into `DataCursor`.
    #[inline]
    pub fn write_u32(&mut self, value: u32) -> Result<(), DataCursorError> {
        const LENGTH: usize = size_of::<u32>();
        if self.pos + LENGTH <= self.data.len() {
            let bytes = match self.endian {
                Endian::Little => value.to_le_bytes(),
                Endian::Big => value.to_be_bytes(),
            };
            unsafe {
                let ptr = self.data.as_mut_ptr().add(self.pos);
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), LENGTH);
            }
            self.pos += LENGTH;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Read four bytes from `DataCursor` and return it as a `i32`.
    #[inline]
    pub fn read_i32(&mut self) -> Result<i32, DataCursorError> {
        self.read_u32().map(|v| v as i32)
    }

    /// Write four bytes from a `i32` into `DataCursor`.
    #[inline]
    pub fn write_i32(&mut self, value: i32) -> Result<(), DataCursorError> {
        self.write_u32(value as u32)
    }

    /// Read eight bytes from `DataCursor` and return it as a `u64`.
    #[inline]
    pub fn read_u64(&mut self) -> Result<u64, DataCursorError> {
        const LENGTH: usize = size_of::<u64>();
        if self.pos + LENGTH <= self.data.len() {
            let value = unsafe {
                let ptr = self.data.as_ptr().add(self.pos).cast::<[u8; LENGTH]>();
                core::ptr::read(ptr)
            };
            self.pos += LENGTH;

            match self.endian {
                Endian::Little => Ok(u64::from_le_bytes(value)),
                Endian::Big => Ok(u64::from_be_bytes(value)),
            }
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Write eight bytes from a `u64` into `DataCursor`.
    #[inline]
    pub fn write_u64(&mut self, value: u64) -> Result<(), DataCursorError> {
        const LENGTH: usize = size_of::<u64>();
        if self.pos + LENGTH <= self.data.len() {
            let bytes = match self.endian {
                Endian::Little => value.to_le_bytes(),
                Endian::Big => value.to_be_bytes(),
            };
            unsafe {
                let ptr = self.data.as_mut_ptr().add(self.pos);
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), LENGTH);
            }
            self.pos += LENGTH;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Read eight bytes from `DataCursor` and return it as a `i64`.
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64, DataCursorError> {
        self.read_u64().map(|v| v as i64)
    }

    /// Write eight bytes from a `i64` into `DataCursor`.
    #[inline]
    pub fn write_i64(&mut self, value: i64) -> Result<(), DataCursorError> {
        self.write_u64(value as u64)
    }

    /// Read sixteen bytes from `DataCursor` and return it as a `u128`.
    #[inline]
    pub fn read_u128(&mut self) -> Result<u128, DataCursorError> {
        const LENGTH: usize = size_of::<u128>();
        if self.pos + LENGTH <= self.data.len() {
            let value = unsafe {
                let ptr = self.data.as_ptr().add(self.pos).cast::<[u8; LENGTH]>();
                core::ptr::read(ptr)
            };
            self.pos += LENGTH;

            match self.endian {
                Endian::Little => Ok(u128::from_le_bytes(value)),
                Endian::Big => Ok(u128::from_be_bytes(value)),
            }
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Write sixteen bytes from a `u128` into `DataCursor`.
    #[inline]
    pub fn write_u128(&mut self, value: u128) -> Result<(), DataCursorError> {
        const LENGTH: usize = size_of::<u128>();
        if self.pos + LENGTH <= self.data.len() {
            let bytes = match self.endian {
                Endian::Little => value.to_le_bytes(),
                Endian::Big => value.to_be_bytes(),
            };
            unsafe {
                let ptr = self.data.as_mut_ptr().add(self.pos);
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), LENGTH);
            }
            self.pos += LENGTH;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Read sixteen bytes from `DataCursor` and return it as a `i128`.
    #[inline]
    pub fn read_i128(&mut self) -> Result<i128, DataCursorError> {
        self.read_u128().map(|v| v as i128)
    }

    /// Write sixteen bytes from a `i128` into `DataCursor`.
    #[inline]
    pub fn write_i128(&mut self, value: i128) -> Result<(), DataCursorError> {
        self.write_u128(value as u128)
    }

    /// Read four bytes from `DataCursor` and return it as a `f32`.
    #[inline]
    pub fn read_f32(&mut self) -> Result<f32, DataCursorError> {
        const LENGTH: usize = size_of::<f32>();
        if self.pos + LENGTH <= self.data.len() {
            let value = unsafe {
                let ptr = self.data.as_ptr().add(self.pos).cast::<[u8; LENGTH]>();
                core::ptr::read(ptr)
            };
            self.pos += LENGTH;

            match self.endian {
                Endian::Little => Ok(f32::from_le_bytes(value)),
                Endian::Big => Ok(f32::from_be_bytes(value)),
            }
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Write four bytes from a `f32` into `DataCursor`.
    #[inline]
    pub fn write_f32(&mut self, value: f32) -> Result<(), DataCursorError> {
        const LENGTH: usize = size_of::<f32>();
        if self.pos + LENGTH <= self.data.len() {
            let bytes = match self.endian {
                Endian::Little => value.to_le_bytes(),
                Endian::Big => value.to_be_bytes(),
            };
            unsafe {
                let ptr = self.data.as_mut_ptr().add(self.pos);
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), LENGTH);
            }
            self.pos += LENGTH;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Read eight bytes from `DataCursor` and return it as a `f64`.
    #[inline]
    pub fn read_f64(&mut self) -> Result<f64, DataCursorError> {
        const LENGTH: usize = size_of::<f64>();
        if self.pos + LENGTH <= self.data.len() {
            let value = unsafe {
                let ptr = self.data.as_ptr().add(self.pos).cast::<[u8; LENGTH]>();
                core::ptr::read(ptr)
            };
            self.pos += LENGTH;

            match self.endian {
                Endian::Little => Ok(f64::from_le_bytes(value)),
                Endian::Big => Ok(f64::from_be_bytes(value)),
            }
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }

    /// Write eight bytes from a `f64` into `DataCursor`.
    #[inline]
    pub fn write_f64(&mut self, value: f64) -> Result<(), DataCursorError> {
        const LENGTH: usize = size_of::<f64>();
        if self.pos + LENGTH <= self.data.len() {
            let bytes = match self.endian {
                Endian::Little => value.to_le_bytes(),
                Endian::Big => value.to_be_bytes(),
            };
            unsafe {
                let ptr = self.data.as_mut_ptr().add(self.pos);
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), LENGTH);
            }
            self.pos += LENGTH;
            Ok(())
        } else {
            Err(DataCursorError::EndOfFile)
        }
    }
}

impl Read for DataCursor {
    /// This function will fill `buf` either fully or until end-of-file.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = core::cmp::min(buf.len(), self.len() - self.pos);
        buf.copy_from_slice(&self.data[self.pos..self.pos + len]);
        self.pos += len;
        Ok(len)
    }

    /// This function will read all bytes until end-of-file and put them in `buf`.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let len = self.len() - self.pos;
        buf.extend_from_slice(&self.data[self.pos..]);
        self.pos = self.len();
        Ok(len)
    }

    /// This function attempts to fill the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) if there
    /// isn't enough data to fill `buf`.
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if self.pos + buf.len() > self.len() {
            Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
        } else {
            self.read(buf).map(|_| ())
        }
    }
}

impl Write for DataCursor {
    /// This function will write `buf` either fully, or until end-of-file.
    ///
    /// # Errors
    /// This function is infallible and will not return an error under any circumstances.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = core::cmp::min(buf.len(), self.len() - self.pos);
        self.data[self.pos..self.pos + len].copy_from_slice(&buf[..len]);
        self.pos += len;
        Ok(len)
    }

    /// `DataCursor` is held entirely in memory, so `flush` is a no-op.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// This function attempts to write the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`WriteZero`](std::io::ErrorKind::WriteZero) if there
    /// isn't enough data to fill `buf`.
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if self.pos + buf.len() > self.len() {
            Err(std::io::Error::from(std::io::ErrorKind::WriteZero))
        } else {
            self.write(buf).map(|_| ())
        }
    }
}

impl BufRead for DataCursor {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(self.remaining_slice())
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }
}

impl From<Box<[u8]>> for DataCursor {
    fn from(value: Box<[u8]>) -> Self {
        Self {
            data: value,
            pos: 0,
            endian: Endian::default(),
        }
    }
}

impl From<Vec<u8>> for DataCursor {
    fn from(value: Vec<u8>) -> Self {
        Self {
            data: value.into_boxed_slice(),
            pos: 0,
            endian: Endian::default(),
        }
    }
}

impl AsRef<[u8]> for DataCursor {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl AsMut<[u8]> for DataCursor {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}
