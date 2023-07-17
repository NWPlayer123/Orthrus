pub mod error;
pub mod time;
pub mod vfs;
pub use crate::error::{Error, Result};
pub use crate::time::{current_time, format_timestamp, TIME_FORMAT};
use std::{
    io,
    io::{Read, Seek, Write},
};

/// Reads a u8 from the input and writes it to the output
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading or writing fails
pub fn copy_byte<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Read + Seek,
    O: Read + Write + Seek,
{
    let mut buffer = [0; 1];
    input.read_exact(&mut buffer)?;
    output.write_all(&buffer)?;
    Ok(())
}

/// Reads a u8 from an input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u8<I>(input: &mut I) -> io::Result<u8>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u8>()];
    input.read_exact(&mut buffer)?;
    Ok(u8::from_be_bytes(buffer))
}

/// Reads a i8 from an input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i8<I>(input: &mut I) -> io::Result<i8>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i8>()];
    input.read_exact(&mut buffer)?;
    Ok(i8::from_be_bytes(buffer))
}

/// Reads a u16 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u16_be<I>(input: &mut I) -> io::Result<u16>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u16>()];
    input.read_exact(&mut buffer)?;
    Ok(u16::from_be_bytes(buffer))
}

/// Reads a i16 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i16_be<I>(input: &mut I) -> io::Result<i16>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i16>()];
    input.read_exact(&mut buffer)?;
    Ok(i16::from_be_bytes(buffer))
}

/// Reads a u32 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u32_be<I>(input: &mut I) -> io::Result<u32>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u32>()];
    input.read_exact(&mut buffer)?;
    Ok(u32::from_be_bytes(buffer))
}

/// Reads a i32 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i32_be<I>(input: &mut I) -> io::Result<i32>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i32>()];
    input.read_exact(&mut buffer)?;
    Ok(i32::from_be_bytes(buffer))
}

/// Reads a f32 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_f32_be<I>(input: &mut I) -> io::Result<f32>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<f32>()];
    input.read_exact(&mut buffer)?;
    Ok(f32::from_be_bytes(buffer))
}

/// Reads a u64 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u64_be<I>(input: &mut I) -> io::Result<u64>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u64>()];
    input.read_exact(&mut buffer)?;
    Ok(u64::from_be_bytes(buffer))
}

/// Reads a i64 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i64_be<I>(input: &mut I) -> io::Result<i64>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i64>()];
    input.read_exact(&mut buffer)?;
    Ok(i64::from_be_bytes(buffer))
}

/// Reads a f64 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_f64_be<I>(input: &mut I) -> io::Result<f64>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<f64>()];
    input.read_exact(&mut buffer)?;
    Ok(f64::from_be_bytes(buffer))
}

/// Reads a u128 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u128_be<I>(input: &mut I) -> io::Result<u128>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u128>()];
    input.read_exact(&mut buffer)?;
    Ok(u128::from_be_bytes(buffer))
}

/// Reads a i128 from a big-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i128_be<I>(input: &mut I) -> io::Result<i128>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i128>()];
    input.read_exact(&mut buffer)?;
    Ok(i128::from_be_bytes(buffer))
}

/// Reads a u16 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u16_le<I>(input: &mut I) -> io::Result<u16>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u16>()];
    input.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

/// Reads a i16 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i16_le<I>(input: &mut I) -> io::Result<i16>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i16>()];
    input.read_exact(&mut buffer)?;
    Ok(i16::from_le_bytes(buffer))
}

/// Reads a u32 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u32_le<I>(input: &mut I) -> io::Result<u32>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u32>()];
    input.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

/// Reads a i32 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i32_le<I>(input: &mut I) -> io::Result<i32>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i32>()];
    input.read_exact(&mut buffer)?;
    Ok(i32::from_le_bytes(buffer))
}

/// Reads a f32 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_f32_le<I>(input: &mut I) -> io::Result<f32>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<f32>()];
    input.read_exact(&mut buffer)?;
    Ok(f32::from_le_bytes(buffer))
}

/// Reads a u64 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u64_le<I>(input: &mut I) -> io::Result<u64>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u64>()];
    input.read_exact(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
}

/// Reads a i64 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i64_le<I>(input: &mut I) -> io::Result<i64>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i64>()];
    input.read_exact(&mut buffer)?;
    Ok(i64::from_le_bytes(buffer))
}

/// Reads a f64 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_f64_le<I>(input: &mut I) -> io::Result<f64>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<f64>()];
    input.read_exact(&mut buffer)?;
    Ok(f64::from_le_bytes(buffer))
}

/// Reads a u128 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_u128_le<I>(input: &mut I) -> io::Result<u128>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<u128>()];
    input.read_exact(&mut buffer)?;
    Ok(u128::from_le_bytes(buffer))
}

/// Reads a i128 from a little-endian input and returns it
///
/// # Errors
///
/// Returns an [`ErrorKind`](std::io::ErrorKind) if reading fails
pub fn read_i128_le<I>(input: &mut I) -> io::Result<i128>
where
    I: Read + Seek,
{
    let mut buffer = [0; core::mem::size_of::<i128>()];
    input.read_exact(&mut buffer)?;
    Ok(i128::from_le_bytes(buffer))
}
