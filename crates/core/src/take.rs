use crate::data::Endian;
use core::ops::{Deref, DerefMut};
use std::io::{Read, Result};

pub struct ByteStream<R: Read> {
    reader: R,
    endian: Endian,
}

impl<R: Read> ByteStream<R> {
    pub fn new(reader: R, endian: Endian) -> Self {
        Self { reader, endian }
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; n];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    pub fn read_exact<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buffer = [0u8; N];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let mut buffer = [0u8; 1];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_exact::<2>()?;
        Ok(match self.endian {
            Endian::Little => u16::from_le_bytes(bytes),
            Endian::Big => u16::from_be_bytes(bytes),
        })
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        let bytes = self.read_exact::<2>()?;
        Ok(match self.endian {
            Endian::Little => i16::from_le_bytes(bytes),
            Endian::Big => i16::from_be_bytes(bytes),
        })
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_exact::<4>()?;
        Ok(match self.endian {
            Endian::Little => u32::from_le_bytes(bytes),
            Endian::Big => u32::from_be_bytes(bytes),
        })
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        let bytes = self.read_exact::<4>()?;
        Ok(match self.endian {
            Endian::Little => i32::from_le_bytes(bytes),
            Endian::Big => i32::from_be_bytes(bytes),
        })
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.read_exact::<8>()?;
        Ok(match self.endian {
            Endian::Little => u64::from_le_bytes(bytes),
            Endian::Big => u64::from_be_bytes(bytes),
        })
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        let bytes = self.read_exact::<8>()?;
        Ok(match self.endian {
            Endian::Little => i64::from_le_bytes(bytes),
            Endian::Big => i64::from_be_bytes(bytes),
        })
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        let bytes = self.read_exact::<4>()?;
        Ok(match self.endian {
            Endian::Little => f32::from_le_bytes(bytes),
            Endian::Big => f32::from_be_bytes(bytes),
        })
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        let bytes = self.read_exact::<8>()?;
        Ok(match self.endian {
            Endian::Little => f64::from_le_bytes(bytes),
            Endian::Big => f64::from_be_bytes(bytes),
        })
    }

    pub fn read_string(&mut self, length: usize) -> Result<String> {
        let bytes = self.read_bytes(length)?;
        String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl<R: Read> Deref for ByteStream<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<R: Read> DerefMut for ByteStream<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}