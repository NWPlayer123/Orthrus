use std::{fs::File, io::Read};

pub fn acquire_file(path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; file.metadata()?.len() as usize];
    file.read(&mut buffer)?;
    Ok(buffer)
}

pub fn read_u16(buffer: &[u8],pos: usize) -> u16 {
    let value: [u8; 2] = buffer[pos .. pos + 2].try_into().unwrap();
    u16::from_be_bytes(value)
}

pub fn read_u32(buffer: &[u8],pos: usize) -> u32 {
    let value: [u8; 4] = buffer[pos .. pos + 4].try_into().unwrap();
    u32::from_be_bytes(value)
}
