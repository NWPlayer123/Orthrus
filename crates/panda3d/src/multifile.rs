use std::fs;
use std::path::Path;
use std::str;
use std::{
    io,
    io::{BufRead, Cursor, Error, ErrorKind, Read, Seek},
};

use orthrus_helper::vfs::VirtualNode;

pub struct Multifile {
    _root: VirtualNode,
}

impl Multifile {
    pub fn new() -> Self {
        Self {
            _root: VirtualNode::new_directory("/".to_string()),
        }
    }

    fn parse_header_prefix<I>(&mut self, input: &mut I) -> io::Result<String>
    where
        I: BufRead + Read + Seek,
    {
        //read each line, and if it starts
        let mut header_prefix = String::new();
        loop {
            let mut line = String::new();
            let len = input.read_line(&mut line)?;

            // reached EOF
            if len == 0 {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to find start of Multifile data!",
                ));
            }

            if line.starts_with('#') {
                header_prefix.push_str(&line);
            } else {
                input.seek(io::SeekFrom::Current(0 - len as i64))?;
                return Ok(header_prefix);
            }
        }
    }

    pub fn open_read(&mut self, path: &Path, offset: u64) -> io::Result<()> {
        let file = fs::read(path)?;
        let mut data = Cursor::new(file);
        data.seek(io::SeekFrom::Start(offset))?;
        //handle special case where it can start with hashtags
        let header_text = self.parse_header_prefix(&mut data)?;
        if !header_text.is_empty() {
            println!("{}", header_text)
        }

        let mut magic = [0u8; 6];
        data.read_exact(&mut magic)?;
        println!("{:?}", str::from_utf8(&magic).unwrap());
        Ok(())
    }
}

impl Default for Multifile {
    fn default() -> Self {
        Self::new()
    }
}
