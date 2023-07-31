use core::mem::size_of;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::path::Path;

/// # Future Compatibility
/// In the future, `DataCursor` will be rewritten with
/// [const_generics](https://github.com/rust-lang/project-const-generics) to reduce boilerplate,
/// along with improving the ergonomics of reading and writing.
pub struct DataCursor {
    data: Box<[u8]>,
    pos: usize,
}

impl DataCursor {
    pub fn new<I: Into<Box<[u8]>>>(data: I) -> Self {
        Self {
            data: data.into(),
            pos: 0,
        }
    }

    /// Constructs a new `DataCursor` using the specified `path`.
    ///
    /// # Errors
    /// This function will return an error if `path` does not exist, if it lacks permission to read
    /// the `metadata` of `path`, or if [`read_exact`](Read::read_exact) is unable to complete.
    pub fn from_path<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let mut file = File::open(path)?;
        let size = file.metadata()?.len() as usize;
        let mut data = vec![0u8; size].into_boxed_slice();
        file.read_exact(&mut data)?;
        Ok(Self { data, pos: 0 })
    }

    /// Read a single byte from the `DataCursor` and write it into another `DataCursor`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data, or
    /// [`ErrorKind::WriteZero`] if there is not enough space in `output`.
    pub fn copy_byte(&mut self, output: &mut Self) -> crate::Result<()> {
        let mut buffer = [0; 1];
        self.read_exact(&mut buffer)?;
        output.write_all(&buffer)?;
        Ok(())
    }

    /// Read a single byte from `DataCursor` and return it as a `u8`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u8(&mut self) -> crate::Result<u8> {
        let mut buffer = [0; size_of::<u8>()];
        self.read_exact(&mut buffer)?;
        Ok(u8::from_be_bytes(buffer))
    }

    /// Read a single byte from `DataCursor` and return it as a `u8`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i8(&mut self) -> crate::Result<i8> {
        let mut buffer = [0; size_of::<i8>()];
        self.read_exact(&mut buffer)?;
        Ok(i8::from_be_bytes(buffer))
    }

    /// Read two bytes from `DataCursor` and return it as a big-endian `u16`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u16_be(&mut self) -> crate::Result<u16> {
        let mut buffer = [0; size_of::<u16>()];
        self.read_exact(&mut buffer)?;
        Ok(u16::from_be_bytes(buffer))
    }

    /// Read two bytes from `DataCursor` and return it as a little-endian `u16`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u16_le(&mut self) -> crate::Result<u16> {
        let mut buffer = [0; size_of::<u16>()];
        self.read_exact(&mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }

    /// Read two bytes from `DataCursor` and return it as a big-endian `i16`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i16_be(&mut self) -> crate::Result<i16> {
        let mut buffer = [0; size_of::<i16>()];
        self.read_exact(&mut buffer)?;
        Ok(i16::from_be_bytes(buffer))
    }

    /// Read two bytes from `DataCursor` and return it as a little-endian `i16`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i16_le(&mut self) -> crate::Result<i16> {
        let mut buffer = [0; size_of::<i16>()];
        self.read_exact(&mut buffer)?;
        Ok(i16::from_le_bytes(buffer))
    }

    /// Read four bytes from `DataCursor` and return it as a big-endian `u32`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u32_be(&mut self) -> crate::Result<u32> {
        let mut buffer = [0; size_of::<u32>()];
        self.read_exact(&mut buffer)?;
        Ok(u32::from_be_bytes(buffer))
    }

    /// Read four bytes from `DataCursor` and return it as a little-endian `u32`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u32_le(&mut self) -> crate::Result<u32> {
        let mut buffer = [0; size_of::<u32>()];
        self.read_exact(&mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    /// Read four bytes from `DataCursor` and return it as a big-endian `i32`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i32_be(&mut self) -> crate::Result<i32> {
        let mut buffer = [0; size_of::<i32>()];
        self.read_exact(&mut buffer)?;
        Ok(i32::from_be_bytes(buffer))
    }

    /// Read four bytes from `DataCursor` and return it as a little-endian `i32`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i32_le(&mut self) -> crate::Result<i32> {
        let mut buffer = [0; size_of::<i32>()];
        self.read_exact(&mut buffer)?;
        Ok(i32::from_le_bytes(buffer))
    }

    /// Read eight bytes from `DataCursor` and return it as a big-endian `u64`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u64_be(&mut self) -> crate::Result<u64> {
        let mut buffer = [0; size_of::<u64>()];
        self.read_exact(&mut buffer)?;
        Ok(u64::from_be_bytes(buffer))
    }

    /// Read eight bytes from `DataCursor` and return it as a little-endian `u64`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u64_le(&mut self) -> crate::Result<u64> {
        let mut buffer = [0; size_of::<u64>()];
        self.read_exact(&mut buffer)?;
        Ok(u64::from_le_bytes(buffer))
    }

    /// Read eight bytes from `DataCursor` and return it as a big-endian `i64`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i64_be(&mut self) -> crate::Result<i64> {
        let mut buffer = [0; size_of::<i64>()];
        self.read_exact(&mut buffer)?;
        Ok(i64::from_be_bytes(buffer))
    }

    /// Read eight bytes from `DataCursor` and return it as a little-endian `i64`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i64_le(&mut self) -> crate::Result<i64> {
        let mut buffer = [0; size_of::<i64>()];
        self.read_exact(&mut buffer)?;
        Ok(i64::from_le_bytes(buffer))
    }

    /// Read sixteen bytes from `DataCursor` and return it as a big-endian `u128`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u128_be(&mut self) -> crate::Result<u128> {
        let mut buffer = [0; size_of::<u128>()];
        self.read_exact(&mut buffer)?;
        Ok(u128::from_be_bytes(buffer))
    }

    /// Read sixteen bytes from `DataCursor` and return it as a little-endian `u128`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_u128_le(&mut self) -> crate::Result<u128> {
        let mut buffer = [0; size_of::<u128>()];
        self.read_exact(&mut buffer)?;
        Ok(u128::from_le_bytes(buffer))
    }

    /// Read sixteen bytes from `DataCursor` and return it as a big-endian `i128`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i128_be(&mut self) -> crate::Result<i128> {
        let mut buffer = [0; size_of::<i128>()];
        self.read_exact(&mut buffer)?;
        Ok(i128::from_be_bytes(buffer))
    }

    /// Read sixteen bytes from `DataCursor` and return it as a little-endian `i128`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_i128_le(&mut self) -> crate::Result<i128> {
        let mut buffer = [0; size_of::<i128>()];
        self.read_exact(&mut buffer)?;
        Ok(i128::from_le_bytes(buffer))
    }

    /// Read four bytes from `DataCursor` and return it as a big-endian `f32`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_f32_be(&mut self) -> crate::Result<f32> {
        let mut buffer = [0; size_of::<f32>()];
        self.read_exact(&mut buffer)?;
        Ok(f32::from_be_bytes(buffer))
    }

    /// Read four bytes from `DataCursor` and return it as a little-endian `f32`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_f32_le(&mut self) -> crate::Result<f32> {
        let mut buffer = [0; size_of::<f32>()];
        self.read_exact(&mut buffer)?;
        Ok(f32::from_le_bytes(buffer))
    }

    /// Read eight bytes from `DataCursor` and return it as a big-endian `f64`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_f64_be(&mut self) -> crate::Result<f64> {
        let mut buffer = [0; size_of::<f64>()];
        self.read_exact(&mut buffer)?;
        Ok(f64::from_be_bytes(buffer))
    }

    /// Read eight bytes from `DataCursor` and return it as a little-endian `f64`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if there is not enough data.
    pub fn read_f64_le(&mut self) -> crate::Result<f64> {
        let mut buffer = [0; size_of::<f64>()];
        self.read_exact(&mut buffer)?;
        Ok(f64::from_le_bytes(buffer))
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn remaining_slice(&self) -> &[u8] {
        let len = self.pos.min(self.data.len());
        &self.data[len..]
    }
}

impl Read for DataCursor {
    /// This function will attempt to read bytes from `DataCursor` into `buf`. It will either fill
    /// the entirety of `buf`, or as many bytes are left until end-of-file.
    ///
    /// # Errors
    /// This function will not return an error under any circumstances.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = core::cmp::min(buf.len(), self.data.len() - self.pos);
        buf.copy_from_slice(&self.data[self.pos..self.pos + len]);
        self.pos += len;
        Ok(len)
    }

    /// This function will read all bytes until end-of-file and put them in `buf`.
    ///
    /// # Errors
    /// This function will not return an error under any circumstances.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let len = self.data.len() - self.pos;
        buf.extend_from_slice(&self.data[self.pos..]);
        self.pos = self.data.len();
        Ok(len)
    }

    /// This function attempts to fill the entirety of `buf`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::UnexpectedEof`] if `DataCursor` doesn't have enough
    /// data to fill `buf`.
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if buf.len() > self.data.len() - self.pos {
            Err(Error::from(ErrorKind::UnexpectedEof))
        } else {
            self.read(buf).map(|_| ())
        }
    }
}

impl Write for DataCursor {
    /// This function will attempt to write `buf` into `DataCursor`. It will either write the
    /// entirety of `buf`, or as much fits until end-of-file.
    ///
    /// # Errors
    /// This function will not return an error under any circumstances.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = core::cmp::min(buf.len(), self.data.len() - self.pos);
        self.data[self.pos..self.pos + len].copy_from_slice(&buf[..len]);
        self.pos += len;
        Ok(len)
    }

    /// `DataCursor` is entirely held in memory, so `flush` is a no-op.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    /// This function attempts to write the entirety of `buf` into `DataCursor`.
    ///
    /// # Errors
    /// This function will return [`ErrorKind::WriteZero`] if there is not enough space to write
    /// `buf`.
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if buf.len() > self.data.len() - self.pos {
            Err(Error::from(ErrorKind::WriteZero))
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
