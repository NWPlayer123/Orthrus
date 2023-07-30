use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;

use enumflags2::{bitflags, BitFlag, BitFlags};
use orthrus_helper::certificate::print_x509_info;
use orthrus_helper::vfs::VirtualNode;
use orthrus_helper::{time, DataCursor, Error, Result};
use x509_parser::prelude::*;

/// This struct is mainly for readability in place of an unnamed tuple
struct Version {
    major: i16,
    minor: i16,
}

/// # Multifile format
/// A Multifile is a container format used by `Panda3D` to distribute assets, similar to a .zip.
///
/// The Multifile specification (currently v1.1, little-endian) is as follows:
///
/// | Offset | Field | Type | Length | Notes |
/// |---|---|---|---|---|
/// | 0 | Magic number | u8\[6\] | 6 | The unique identifier to let us know we're reading a Multifile (always "pmf\0\n\r"). |
/// | 6 | Major version | i16 | 2 | The major version of the Multifile format (currently 1). |
/// | 8 | Minor version | i16 | 2 | The minor version of the Multifile format (currently 1). |
/// | 10 | Scale factor | u32 | 4 | To allow for Multifiles larger than a 32-bit integer (4,294,967,295 bytes), this will "scale" all offsets. |
/// | 14 | Unix timestamp | u32 | 4 | The date the Multifile was last modified. |
///
/// ## Subfile Format
/// Following the Multifile header is a number of Subfiles. In order to parse the Multifile body,
/// you read an index offset pointing to the next Subfile header and follow it until you reach a 0
/// which indicates the end of the Multifile.
///
/// The Subfile format (v1.1) is as follows:
///
/// | Offset | Field | Type | Length | Notes |
/// |---|---|---|---|---|
/// | 0 | Index offset | u32 | 4 | Offset to the next header relative to file start. Multiply by the scale factor to get the correct offset. If it's 0, there are no more Subfiles. |
/// | 4 | Data offset | u32 | 4 | Offset to the actual Subfile data relative to file start. Multiply by the scale factor to get the correct offset. |
/// | 8 | Data length | u32 | 4 | The length of the Subfile data. |
/// | 12 | Subfile flags | u16 | 2 | These are special attributes attached to a Subfile, such as compression or encryption. See [Subfile flags](#subfile-flags) for a complete list. |
/// | 14 | Original length | u32 | 4 | Only in the Subfile header if the Subfile is compressed or encrypted! This is the length of the data after being decompressed/decrypted. |
/// | 18 | Unix timestamp | u32 | 4 | The date the Subfile was last modified. If it's 0, use the Multifile timestamp. |
/// | 20 | Name length | u16 | 2 | Length of the Subfile's name. Read this many more bytes. |
/// | 22 | Subfile path | char\[length\] | length | This is the path to the actual Subfile, used as part of a Virtual Filesystem. Obfuscated, convert each character as (255 - x). |
///
/// ## Subfile Flags
/// | Flag | Value | Notes |
/// |---|---|---|
/// | `Deleted` | 1 << 0 (1) | The Subfile has been deleted from the Multifile, so it should be ignored. |
/// | `IndexInvalid` | 1 << 1 (2) | The Subfile has a corrupt index entry. |
/// | `DataInvalid` | 1 << 2 (4) | The Subfile has invalid data. |
/// | `Compressed` | 1 << 3 (8) | The Subfile is compressed. |
/// | `Encrypted` | 1 << 4 (16) | The Subfile is encrypted. |
/// | `Signature` | 1 << 5 (32) | The Subfile is the certificate used to sign the Multifile. See [Certificate format](#certificate-format) for more details. |
/// | `Text` | 1 << 6 (64) | The Subfile contains text instead of binary data. |
///
/// ## Certificate Format
/// The Multifile certificate is a binary blob that can contain multiple certificates, along with
/// the actual signature of the Multifile. It starts with the length of the signature (u32), and
/// then the actual signature. After that is the number of certificates (u32), and then a blob
/// containing all certificates. The way it is meant to be read is to repeatedly call `d2i_X509` or
/// an equivalent function that allows you to get the remaining bytes after parsing a certificate.
pub struct Multifile {
    _root: VirtualNode,
    version: Version,
    scale_factor: u32,
    timestamp: u32,
}

impl Multifile {
    const CURRENT_MAJOR_VER: i16 = 1;
    const CURRENT_MINOR_VER: i16 = 1;
    const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    #[must_use]
    pub fn new() -> Self {
        Self {
            _root: VirtualNode::new_directory("/".to_string()),
            version: Version { major: 0, minor: 0 },
            scale_factor: 1,
            timestamp: 0,
        }
    }

    /// Parses a `Panda3D` Multifile pre-header, which allows for comment lines starting with '#'.
    ///
    /// Returns either a [String] containing the header comment data, or an [`io::Error`] if it
    /// reaches EOF before finding the Multifile magic ("pmf\0\n\r").
    fn parse_header_prefix(input: &mut DataCursor) -> Result<String> {
        let mut header_prefix = String::new();
        loop {
            let mut line = String::new();
            let len = input.read_line(&mut line)?;

            // reached EOF
            if len == 0 {
                return Err(Error::EndOfFile);
            }

            // check if this is a header comment, otherwise return comment data
            if line.starts_with('#') {
                header_prefix.push_str(&line);
            } else {
                input.seek(SeekFrom::Current(0 - len as i64))?;
                return Ok(header_prefix);
            }
        }
    }

    #[must_use]
    pub fn current_version(&self) -> String {
        format!("{}.{}", Self::CURRENT_MAJOR_VER, Self::CURRENT_MINOR_VER)
    }

    #[must_use]
    pub fn version(&self) -> String {
        format!("{}.{}", self.version.major, self.version.minor)
    }

    pub fn open_read(&mut self, path: &Path, offset: u64) -> Result<()> {
        //acquire file data
        let mut data = DataCursor::new_from_file(path)?;
        data.seek(SeekFrom::Start(offset))?;

        //handle special case where it can start with hashtags
        let header_text = Self::parse_header_prefix(&mut data)?;
        if !header_text.is_empty() {
            log::info!("Multifile pre-header:\n{}\n", header_text);
        }

        //check Multifile magic
        let mut magic = [0u8; 6];
        data.read_exact(&mut magic)?;

        if magic != Self::MAGIC {
            let error = Error::InvalidMagic {
                expected: format!("{:?}", std::str::from_utf8(&Self::MAGIC)?),
                got: format!("{:?}", std::str::from_utf8(&magic)?),
            };
            log::error!("{}", error.to_string());
            return Err(error);
        }

        //start reading header
        self.version = Version {
            major: data.read_i16_le()?,
            minor: data.read_i16_le()?,
        };
        log::info!("Multifile version v{}", self.version());

        if self.version.major != Self::CURRENT_MAJOR_VER
            || self.version.minor > Self::CURRENT_MINOR_VER
        {
            let error = Error::UnknownVersion {
                expected: self.current_version(),
                got: self.version(),
            };
            log::error!("{}", error.to_string());
            return Err(error);
        }

        self.scale_factor = data.read_u32_le()?;
        log::info!("Scale factor (for >4GB files): {}", self.scale_factor);

        if self.version.minor >= 1 {
            self.timestamp = data.read_u32_le()?;
            log::info!(
                "File Unix timestamp: {} {}",
                self.timestamp,
                time::format_timestamp(i64::from(self.timestamp))?
            );
        }

        //Subfile loop, separate function probably
        let mut next_index = data.read_u32_le()? * self.scale_factor;
        while next_index != 0 {
            let mut subfile = Subfile::new();

            subfile.offset = data.read_u32_le()? * self.scale_factor;
            subfile.data_length = data.read_u32_le()?;
            subfile.flags = BitFlags::<SubfileFlags>::from_bits_truncate(data.read_u16_le()?);
            log::debug!(
                "Data offset: {:#X} | Data Length: {:#X} | Subfile flags: {}",
                subfile.offset,
                subfile.data_length,
                subfile.flags
            );

            if subfile
                .flags
                .intersects(SubfileFlags::Compressed | SubfileFlags::Encrypted)
            {
                subfile.length = data.read_u32_le()?;
                log::debug!(
                    "Subfile is compressed or encrypted! Original length: {}",
                    subfile.length
                );
            } else {
                subfile.length = subfile.data_length;
            }

            let timestamp: u32 = data.read_u32_le()?;
            //if the subfile timestamp is 0, use the global timestamp
            if timestamp == 0 {
                subfile.timestamp = self.timestamp;
            }

            let name_length = data.read_u16_le()?;
            subfile.filename.reserve_exact(name_length.into());

            for _ in 0..name_length {
                subfile.filename.push((255 - data.read_u8()?) as char);
            }

            if !subfile.filename.is_empty() {
                log::debug!("Subfile name: {}", subfile.filename);
            }
            log::debug!(""); //new line to make it easier to read

            if subfile.flags.contains(SubfileFlags::Signature) {
                data.seek(SeekFrom::Start(subfile.offset.into()))?;

                let mut certificate = DataCursor::new(vec![0u8; subfile.length as usize]);
                data.read_exact(certificate.as_mut_slice())?;
                let signature_length = certificate.read_u32_le()?;
                certificate.seek(SeekFrom::Current(signature_length.into()))?;
                let num_certificates = certificate.read_u32_le()?;

                let mut certificate_data = certificate.as_slice();
                for certificate_number in 1..=num_certificates {
                    log::debug!("Certificate {}", certificate_number);
                    let (rest, cert) = X509Certificate::from_der(certificate_data)?;
                    certificate_data = rest;
                    print_x509_info(&cert)?;
                    log::debug!("");
                }
            }

            data.seek(SeekFrom::Start(next_index.into()))?;
            next_index = data.read_u32_le()? * self.scale_factor;
        }
        Ok(())
    }
}

impl Default for Multifile {
    fn default() -> Self {
        Self::new()
    }
}

#[bitflags]
#[repr(u16)]
#[derive(Copy, Clone, Debug)]
enum SubfileFlags {
    Deleted = 1 << 0,
    IndexInvalid = 1 << 1,
    DataInvalid = 1 << 2,
    Compressed = 1 << 3,
    Encrypted = 1 << 4,
    Signature = 1 << 5,
    Text = 1 << 6,
}

struct Subfile {
    offset: u32,
    data_length: u32,
    flags: BitFlags<SubfileFlags, u16>,
    length: u32,
    timestamp: u32,
    filename: String,
}

impl Subfile {
    #[must_use]
    pub fn new() -> Self {
        Self {
            offset: 0,
            data_length: 0,
            flags: SubfileFlags::empty(),
            length: 0,
            timestamp: 0,
            filename: String::new(),
        }
    }
}
