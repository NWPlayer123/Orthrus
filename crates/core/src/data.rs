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
        // Perform a single bounds check to know if we can safely copy raw
        self.data
            .get(src..src + length)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.data
            .get(self.pos..self.pos + length)
            .ok_or_else(|| DataCursorError::EndOfFile)?;

        for n in 0..length {
            self.data[src + n] = self.data[self.pos + n];
        }
        self.pos += length;

        Ok(())
    }

    /// Read one byte from `DataCursor` and return it as a `u8`.
    pub fn read_u8(&mut self) -> Result<u8, DataCursorError> {
        let value = self
            .data
            .get(self.pos)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += 1;
        Ok(*value)
    }

    /// Write one byte from a `u8` into `DataCursor`.
    pub fn write_u8(&mut self, value: u8) -> Result<(), DataCursorError> {
        let target = self
            .data
            .get_mut(self.pos)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        *target = value;
        self.pos += 1;
        Ok(())
    }

    /// Read one byte from `DataCursor` and return it as a `i8`.
    pub fn read_i8(&mut self) -> Result<i8, DataCursorError> {
        let value = self
            .data
            .get(self.pos)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += 1;
        Ok(*value as i8)
    }

    /// Write one byte from a `i8` into `DataCursor`.
    pub fn write_i8(&mut self, value: i8) -> Result<(), DataCursorError> {
        let target = self
            .data
            .get_mut(self.pos)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        *target = value as u8;
        self.pos += 1;
        Ok(())
    }

    /// Read two bytes from `DataCursor` and return it as a `u16`.
    pub fn read_u16(&mut self) -> Result<u16, DataCursorError> {
        let len = size_of::<u16>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [bytes[0], bytes[1]];
        match self.endian {
            Endian::Little => Ok(u16::from_le_bytes(array)),
            Endian::Big => Ok(u16::from_be_bytes(array)),
        }
    }

    /// Write two bytes from a `u16` into `DataCursor`.
    pub fn write_u16(&mut self, value: u16) -> Result<(), DataCursorError> {
        let len = size_of::<u16>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read two bytes from `DataCursor` and return it as a `i16`.
    pub fn read_i16(&mut self) -> Result<i16, DataCursorError> {
        let len = size_of::<i16>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [bytes[0], bytes[1]];
        match self.endian {
            Endian::Little => Ok(i16::from_le_bytes(array)),
            Endian::Big => Ok(i16::from_be_bytes(array)),
        }
    }

    /// Write two bytes from a `i16` into `DataCursor`.
    pub fn write_i16(&mut self, value: i16) -> Result<(), DataCursorError> {
        let len = size_of::<i16>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read four bytes from `DataCursor` and return it as a `u32`.
    pub fn read_u32(&mut self) -> Result<u32, DataCursorError> {
        let len = size_of::<u32>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match self.endian {
            Endian::Little => Ok(u32::from_le_bytes(array)),
            Endian::Big => Ok(u32::from_be_bytes(array)),
        }
    }

    /// Write four bytes from a `u32` into `DataCursor`.
    pub fn write_u32(&mut self, value: u32) -> Result<(), DataCursorError> {
        let len = size_of::<u32>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read four bytes from `DataCursor` and return it as a `i32`.
    pub fn read_i32(&mut self) -> Result<i32, DataCursorError> {
        let len = size_of::<i32>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match self.endian {
            Endian::Little => Ok(i32::from_le_bytes(array)),
            Endian::Big => Ok(i32::from_be_bytes(array)),
        }
    }

    /// Write four bytes from a `i32` into `DataCursor`.
    pub fn write_i32(&mut self, value: i32) -> Result<(), DataCursorError> {
        let len = size_of::<i32>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read eight bytes from `DataCursor` and return it as a `u64`.
    pub fn read_u64(&mut self) -> Result<u64, DataCursorError> {
        let len = size_of::<u64>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ];
        match self.endian {
            Endian::Little => Ok(u64::from_le_bytes(array)),
            Endian::Big => Ok(u64::from_be_bytes(array)),
        }
    }

    /// Write eight bytes from a `u64` into `DataCursor`.
    pub fn write_u64(&mut self, value: u64) -> Result<(), DataCursorError> {
        let len = size_of::<u64>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read eight bytes from `DataCursor` and return it as a `i64`.
    pub fn read_i64(&mut self) -> Result<i64, DataCursorError> {
        let len = size_of::<i64>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ];
        match self.endian {
            Endian::Little => Ok(i64::from_le_bytes(array)),
            Endian::Big => Ok(i64::from_be_bytes(array)),
        }
    }

    /// Write eight bytes from a `i64` into `DataCursor`.
    pub fn write_i64(&mut self, value: i64) -> Result<(), DataCursorError> {
        let len = size_of::<i64>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read sixteen bytes from `DataCursor` and return it as a `u128`.
    pub fn read_u128(&mut self) -> Result<u128, DataCursorError> {
        let len = size_of::<u128>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ];
        match self.endian {
            Endian::Little => Ok(u128::from_le_bytes(array)),
            Endian::Big => Ok(u128::from_be_bytes(array)),
        }
    }

    /// Write sixteen bytes from a `u128` into `DataCursor`.
    pub fn write_u128(&mut self, value: u128) -> Result<(), DataCursorError> {
        let len = size_of::<u128>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read sixteen bytes from `DataCursor` and return it as a `i128`.
    pub fn read_i128(&mut self) -> Result<i128, DataCursorError> {
        let len = size_of::<i128>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ];
        match self.endian {
            Endian::Little => Ok(i128::from_le_bytes(array)),
            Endian::Big => Ok(i128::from_be_bytes(array)),
        }
    }

    /// Write sixteen bytes from a `i128` into `DataCursor`.
    pub fn write_i128(&mut self, value: i128) -> Result<(), DataCursorError> {
        let len = size_of::<i128>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read four bytes from `DataCursor` and return it as a `f32`.
    pub fn read_f32(&mut self) -> Result<f32, DataCursorError> {
        let len = size_of::<f32>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match self.endian {
            Endian::Little => Ok(f32::from_le_bytes(array)),
            Endian::Big => Ok(f32::from_be_bytes(array)),
        }
    }

    /// Write four bytes from a `f32` into `DataCursor`.
    pub fn write_f32(&mut self, value: f32) -> Result<(), DataCursorError> {
        let len = size_of::<f32>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
    }

    /// Read eight bytes from `DataCursor` and return it as a `f64`.
    pub fn read_f64(&mut self) -> Result<f64, DataCursorError> {
        let len = size_of::<f64>();
        let bytes = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        let array = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ];
        match self.endian {
            Endian::Little => Ok(f64::from_le_bytes(array)),
            Endian::Big => Ok(f64::from_be_bytes(array)),
        }
    }

    /// Write eight bytes from a `f64` into `DataCursor`.
    pub fn write_f64(&mut self, value: f64) -> Result<(), DataCursorError> {
        let len = size_of::<f64>();
        let data = self
            .data
            .get_mut(self.pos..self.pos + len)
            .ok_or_else(|| DataCursorError::EndOfFile)?;
        self.pos += len;

        match self.endian {
            Endian::Little => data.copy_from_slice(&value.to_le_bytes()),
            Endian::Big => data.copy_from_slice(&value.to_be_bytes()),
        }
        Ok(())
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
        if self.pos + buf.len() >= self.len() {
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
        if self.pos + buf.len() >= self.len() {
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
