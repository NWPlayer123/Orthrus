use std::io::prelude::*;
use std::io::Cursor;
use std::path::Path;

pub struct DataCursor {
    inner: Cursor<Vec<u8>>,
}

impl DataCursor {
    #[must_use]
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            inner: Cursor::new(data),
        }
    }

    /// Reads data from a file into memory and returns a [`DataCursor`]
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to open the file or read it into memory
    pub fn new_from_file<P>(path: P) -> crate::Result<Self>
    where
        P: AsRef<Path>,
    {
        let data = std::fs::read(path)?;
        Ok(Self::new(data))
    }

    /// Reads a u8 and writes it to the output
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read or write enough bytes
    pub fn copy_byte(&mut self, output: &mut Self) -> crate::Result<()> {
        let mut buffer = [0; 1];
        self.inner.read_exact(&mut buffer)?;
        output.write_all(&buffer)?;
        Ok(())
    }

    /// Reads a u8 from an input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u8(&mut self) -> crate::Result<u8> {
        let mut buffer = [0; core::mem::size_of::<u8>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u8::from_be_bytes(buffer))
    }

    /// Reads a i8 from an input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i8(&mut self) -> crate::Result<i8> {
        let mut buffer = [0; core::mem::size_of::<i8>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i8::from_be_bytes(buffer))
    }

    /// Reads a u16 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u16_be(&mut self) -> crate::Result<u16> {
        let mut buffer = [0; core::mem::size_of::<u16>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u16::from_be_bytes(buffer))
    }

    /// Reads a i16 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i16_be(&mut self) -> crate::Result<i16> {
        let mut buffer = [0; core::mem::size_of::<i16>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i16::from_be_bytes(buffer))
    }

    /// Reads a u32 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u32_be(&mut self) -> crate::Result<u32> {
        let mut buffer = [0; core::mem::size_of::<u32>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u32::from_be_bytes(buffer))
    }

    /// Reads a i32 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i32_be(&mut self) -> crate::Result<i32> {
        let mut buffer = [0; core::mem::size_of::<i32>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i32::from_be_bytes(buffer))
    }

    /// Reads a f32 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_f32_be(&mut self) -> crate::Result<f32> {
        let mut buffer = [0; core::mem::size_of::<f32>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(f32::from_be_bytes(buffer))
    }

    /// Reads a u64 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u64_be(&mut self) -> crate::Result<u64> {
        let mut buffer = [0; core::mem::size_of::<u64>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u64::from_be_bytes(buffer))
    }

    /// Reads a i64 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i64_be(&mut self) -> crate::Result<i64> {
        let mut buffer = [0; core::mem::size_of::<i64>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i64::from_be_bytes(buffer))
    }

    /// Reads a f64 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_f64_be(&mut self) -> crate::Result<f64> {
        let mut buffer = [0; core::mem::size_of::<f64>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(f64::from_be_bytes(buffer))
    }

    /// Reads a u128 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u128_be(&mut self) -> crate::Result<u128> {
        let mut buffer = [0; core::mem::size_of::<u128>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u128::from_be_bytes(buffer))
    }

    /// Reads a i128 from a big-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i128_be(&mut self) -> crate::Result<i128> {
        let mut buffer = [0; core::mem::size_of::<i128>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i128::from_be_bytes(buffer))
    }

    /// Reads a u16 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u16_le(&mut self) -> crate::Result<u16> {
        let mut buffer = [0; core::mem::size_of::<u16>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u16::from_le_bytes(buffer))
    }

    /// Reads a i16 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i16_le(&mut self) -> crate::Result<i16> {
        let mut buffer = [0; core::mem::size_of::<i16>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i16::from_le_bytes(buffer))
    }

    /// Reads a u32 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u32_le(&mut self) -> crate::Result<u32> {
        let mut buffer = [0; core::mem::size_of::<u32>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    /// Reads a i32 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i32_le(&mut self) -> crate::Result<i32> {
        let mut buffer = [0; core::mem::size_of::<i32>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i32::from_le_bytes(buffer))
    }

    /// Reads a f32 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_f32_le(&mut self) -> crate::Result<f32> {
        let mut buffer = [0; core::mem::size_of::<f32>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(f32::from_le_bytes(buffer))
    }

    /// Reads a u64 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u64_le(&mut self) -> crate::Result<u64> {
        let mut buffer = [0; core::mem::size_of::<u64>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u64::from_le_bytes(buffer))
    }

    /// Reads a i64 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i64_le(&mut self) -> crate::Result<i64> {
        let mut buffer = [0; core::mem::size_of::<i64>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i64::from_le_bytes(buffer))
    }

    /// Reads a f64 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_f64_le(&mut self) -> crate::Result<f64> {
        let mut buffer = [0; core::mem::size_of::<f64>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(f64::from_le_bytes(buffer))
    }

    /// Reads a u128 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_u128_le(&mut self) -> crate::Result<u128> {
        let mut buffer = [0; core::mem::size_of::<u128>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(u128::from_le_bytes(buffer))
    }

    /// Reads a i128 from a little-endian input and returns it
    ///
    /// # Errors
    ///
    /// Returns an [`IOError`](crate::Error::Io) if unable to read enough bytes
    pub fn read_i128_le(&mut self) -> crate::Result<i128> {
        let mut buffer = [0; core::mem::size_of::<i128>()];
        self.inner.read_exact(&mut buffer)?;
        Ok(i128::from_le_bytes(buffer))
    }

    #[must_use]
    pub fn get_ref(&self) -> &Vec<u8> {
        self.inner.get_ref()
    }

    pub fn as_slice(&mut self) -> &[u8] {
        self.inner.get_ref().as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.inner.get_mut().as_mut_slice()
    }

    #[must_use]
    pub fn position(&self) -> u64 {
        self.inner.position()
    }
}

impl Read for DataCursor {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.inner.read_exact(buf)
    }
}

impl Write for DataCursor {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner.write_all(buf)
    }
}

impl BufRead for DataCursor {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}

impl Seek for DataCursor {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
