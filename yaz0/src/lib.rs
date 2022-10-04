use orthrus_helper as orthrus;

/// This is designed so that the only thing that persists is either an error of why
/// decompression failed, or a buffer containing the decompressed data.
/// Yaz0 is thin enough that it isn't worth saving any data from the original file.
pub fn load(path: &str) -> Result<Vec<u8>, std::io::Error> {
    //try to open file, create a buffer around it
    let mut input = orthrus::acquire_file(path)?;

    //read header from the buffer
    let _magic = orthrus::read_u32(&input, 0);
    let dec_size: usize = orthrus::read_u32(&input, 4) as usize;
    let _alignment = orthrus::read_u32(&input, 8);

    //allocate decompression buffer, zero-initialize
    let mut buffer = vec![0u8; dec_size];

    //perform the actual decompression
    decompress_into(input.as_mut_slice(), buffer.as_mut_slice(), dec_size)?;

    //if we've gotten this far, buffer is the valid decompressed data
    Ok(buffer)
}

fn decompress_into(input: &mut [u8], output: &mut [u8], output_size: usize) -> Result<(), std::io::Error> {
    let mut src_pos: usize = 0x10;
    let mut dst_pos: usize = 0;
    let mut mask: u8 = 0;
    let mut flags: u8 = 0;

    while dst_pos < output_size {
        //if we're out of bits for RLE, load in a new byte
        if mask == 0 {
            mask = 1 << 7;
            flags = input[src_pos];
            src_pos += 1;
        }

        if (flags & mask) != 0 { //read one byte
            output[dst_pos] = input[src_pos];
            src_pos += 1;
            dst_pos += 1;
        }
        else {
            let code = orthrus::read_u16(input, src_pos);
            src_pos += 2;

            let back = ((code & 0xFFF) + 1) as usize;
            let size: usize = match code >> 12 {
                0 => { let n = input[src_pos] as usize; src_pos += 1; n + 0x12 }
                n => n as usize + 2
            };
            for n in dst_pos .. dst_pos + size {
                output[n] = output[n - back];
            }
            dst_pos += size;
        }

        mask >>= 1;
    }
    Ok(())
}

//note to self: for decompression algo, check if the min size is even possible (2), check max (0x111),
//anywhere in the 0x1000 runback and then bisect until we find the minimum (0x88, 0x44, 0xAA, etc)
