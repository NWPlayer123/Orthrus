use orthrus_helper as orthrus;
use std::fs;
use std::{
    io,
    io::{Cursor, Read, Seek, Write},
};

/// Loads the file at `path` and tries to decompress it as a Yaz0 file.
///
/// # Errors
///
/// Returns a [`std::io::Error`] if `path` does not exist, or read/write fails.
///
/// # Panics
///
/// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
pub fn decompress(path: &str) -> io::Result<Cursor<Vec<u8>>> {
    //acquire file data
    let data = fs::read(path)?;
    let mut input = Cursor::new(data);

    //read header from the buffer
    let _magic = orthrus::read_u32_be(&mut input)?; //"Yaz0"
    let dec_size = orthrus::read_u32_be(&mut input)?;
    let _alignment = orthrus::read_u32_be(&mut input)?; //0 on GC/Wii files

    //allocate decompression buffer
    let buffer = vec![0u8; dec_size as usize];
    let mut output = Cursor::new(buffer);

    //perform the actual decompression
    decompress_into(&mut input, &mut output)?;

    //if we've gotten this far, buffer is the valid decompressed data
    Ok(output)
}

/// Decompresses a Yaz0 file into the output buffer.
///
/// This function makes no guarantees about the validity of the Yaz0 stream. It
/// requires that input is a valid Yaz0 file including the header, and that
/// output is large enough to write the decompressed data into.
///
/// # Errors
///
/// Returns a [`std::io::Error`] if read/write fails.
///
/// # Panics
///
/// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
//#[inline(never)]
fn decompress_into<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Read + Seek,
    O: Read + Write + Seek,
{
    let mut mask: u8 = 0;
    let mut flags: u8 = 0;

    //align input to start of compressed Yaz0 stream
    input.seek(io::SeekFrom::Start(0x10))?;
    //get size of output buffer for our loop, align to start of buffer
    let output_size: u64 = output.seek(io::SeekFrom::End(0))?;
    output.seek(io::SeekFrom::Start(0))?;

    while output.stream_position()? < output_size {
        //out of flag bits for RLE, load in a new byte
        if mask == 0 {
            mask = 1 << 7;
            flags = orthrus::read_u8(input)?;
        }

        if (flags & mask) != 0 {
            //copy one byte
            orthrus::copy_byte(input, output)?;
        } else {
            //do RLE copy
            let code = orthrus::read_u16_be(input)?;

            let back = ((code & 0xFFF) + 1) as u64;
            let size = match code >> 12 {
                0 => orthrus::read_u8(input)? as u64 + 0x12,
                n => n as u64 + 2,
            };

            //the ranges can overlap so we need to copy byte-by-byte
            let mut temp = [0u8; 1];
            let position = output.stream_position()?;
            for n in position..position + size {
                output.seek(io::SeekFrom::Start(n - back))?;
                temp[0] = orthrus::read_u8(output)?;
                output.seek(io::SeekFrom::Start(n))?;
                output.write_all(&temp)?;
            }
        }

        mask >>= 1;
    }
    Ok(())
}

//note to self: for compression algo, check if the min size is even possible (2), check max (0x111),
//anywhere in the 0x1000 runback and then bisect until we find the minimum (0x88, 0x44, 0xAA, etc)
