use std::fs;
use std::path::Path;
use std::str;
use std::{
    io,
    io::{BufRead, Cursor, ErrorKind, Read, Seek},
};

use orthrus_helper as orthrus;
use orthrus_helper::vfs::VirtualNode;
use orthrus_helper::{Error, Result};

pub struct Multifile {
    _root: VirtualNode,
}

impl Multifile {
    const CURRENT_MAJOR_VER: i16 = 1;
    const CURRENT_MINOR_VER: i16 = 1;
    pub fn new() -> Self {
        Self {
            _root: VirtualNode::new_directory("/".to_string()),
        }
    }

    /// Parses a Panda3D Multifile pre-header, which allows for comment lines
    /// starting with '#'.
    ///
    /// Returns either a [String] containing the header comment data, or an
    /// [io::Error] if it reaches EOF before finding the Multifile magic
    /// ("pmf\0\n\r").
    fn parse_header_prefix<I>(&mut self, input: &mut I) -> io::Result<String>
    where
        I: BufRead + Read + Seek,
    {
        let mut header_prefix = String::new();
        loop {
            let mut line = String::new();
            let len = input.read_line(&mut line)?;

            // reached EOF
            if len == 0 {
                return Err(io::Error::new(
                    ErrorKind::Other,
                    "Failed to find start of Multifile data!",
                ));
            }

            // check if this is a header comment, otherwise return comment data
            if line.starts_with('#') {
                header_prefix.push_str(&line);
            } else {
                input.seek(io::SeekFrom::Current(0 - len as i64))?;
                return Ok(header_prefix);
            }
        }
    }

    pub fn open_read(&mut self, path: &Path, offset: u64) -> Result<()> {
        let file = fs::read(path)?;
        let mut data = Cursor::new(file);
        data.seek(io::SeekFrom::Start(offset))?;
        //handle special case where it can start with hashtags
        let header_text = self.parse_header_prefix(&mut data)?;
        if !header_text.is_empty() {
            log::info!("Multifile pre-header:\n{}\n", header_text)
        }

        let mut magic = [0u8; 6];
        data.read_exact(&mut magic)?;

        if magic != [112, 109, 102, 0, 10, 13] {
            let error = Error::InvalidMagic {
                expected: format!("{:?}", "pmf\0\n\r"),
                got: format!("{:?}", str::from_utf8(&magic)?),
            };
            log::error!("{}", error.to_string());
            return Err(error);
        }

        let major_version = orthrus::read_i16_le(&mut data)?;
        let minor_version = orthrus::read_i16_le(&mut data)?;

        log::info!("Multifile version v{}.{}", major_version, minor_version);

        if major_version != Multifile::CURRENT_MAJOR_VER
            || minor_version > Multifile::CURRENT_MINOR_VER
        {
            let error = Error::UnknownVersion {
                expected: format!(
                    "{}.{}",
                    Multifile::CURRENT_MAJOR_VER,
                    Multifile::CURRENT_MINOR_VER
                ),
                got: format!("{}.{}", major_version, minor_version),
            };
            log::error!("{}", error.to_string());
            return Err(error);
        }

        let scale_factor = orthrus::read_u32_le(&mut data)?;
        log::info!("Scale factor (for >4GB files): {}", scale_factor);

        if minor_version >= 1 {
            let timestamp = orthrus::read_u32_le(&mut data)?;
            log::info!(
                "File Unix timestamp: {} {}",
                timestamp,
                orthrus::format_timestamp(timestamp as i64)?
            );
        }

        //Subfile loop, separate function probably
        Ok(())
    }
}

impl Default for Multifile {
    fn default() -> Self {
        Self::new()
    }
}
