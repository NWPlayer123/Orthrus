use orthrus_helper::DataCursor;
use orthrus_helper::Result;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;

/// Loads the file at `path` and tries to decompress it as a Yaz0 file.
///
/// # Errors
///
/// Returns a [`std::io::Error`] if `path` does not exist, or read/write fails.
///
/// # Panics
///
/// Panics if the Yaz0 stream is malformed and it tries to read past file bounds.
pub fn decompress_from_path<P>(path: P) -> Result<DataCursor>
where
    P: AsRef<Path>,
{
    //acquire file data, return an error if we can't
    let mut input = DataCursor::new_from_file(path)?;

    //read header from the buffer
    let _magic = input.read_u32_be()?; //"Yaz0"
    let dec_size = input.read_u32_be()?;
    let _alignment = input.read_u32_be()?; //0 on GC/Wii files

    //allocate decompression buffer
    let mut output = DataCursor::new(vec![0u8; dec_size as usize]);

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
fn decompress_into(input: &mut DataCursor, output: &mut DataCursor) -> Result<()> {
    let mut mask: u8 = 0;
    let mut flags: u8 = 0;

    //align input to start of compressed Yaz0 stream
    input.seek(SeekFrom::Start(0x10))?;
    //get size of output buffer for our loop, align to start of buffer
    let output_size: u64 = output.seek(SeekFrom::End(0))?;
    output.seek(SeekFrom::Start(0))?;

    while output.stream_position()? < output_size {
        //out of flag bits for RLE, load in a new byte
        if mask == 0 {
            mask = 1 << 7;
            flags = input.read_u8()?;
        }

        if (flags & mask) == 0 {
            //do RLE copy
            let code = input.read_u16_be()?;

            let back = u64::from((code & 0xFFF) + 1);
            let size = match code >> 12 {
                0 => u64::from(input.read_u8()?) + 0x12,
                n => u64::from(n) + 2,
            };

            //the ranges can overlap so we need to copy byte-by-byte
            let mut temp = [0u8; 1];
            let position = output.stream_position()?;
            for n in position..position + size {
                output.seek(SeekFrom::Start(n - back))?;
                temp[0] = output.read_u8()?;
                output.seek(SeekFrom::Start(n))?;
                output.write_all(&temp)?;
            }
        } else {
            //copy one byte
            input.copy_byte(output)?;
        }

        mask >>= 1;
    }
    Ok(())
}

//note to self: for compression algo, check if the min size is even possible (2), check max (0x111),
//anywhere in the 0x1000 runback and then bisect until we find the minimum (0x88, 0x44, 0xAA, etc)
