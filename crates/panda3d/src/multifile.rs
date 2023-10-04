use core::fmt;
use core::str::from_utf8;
use std::io::prelude::*;
use std::path::Path;

use bitflags::bitflags;
use compact_str::CompactString;
use orthrus_core::certificate::{print_x509_info, Certificate};
use orthrus_core::prelude::*;
use orthrus_core::time;
use orthrus_core::vfs::VirtualFolder;

/// This struct is mainly for readability in place of an unnamed tuple
#[derive(PartialEq)]
struct Version {
    major: i16,
    minor: i16,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A Multifile is a container format used by `Panda3D` to distribute assets, similar to a .zip.
///
/// # Multifile format
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
/// | 22 | Subfile path | char\[len\] | len | This is the path to the actual Subfile, used as part of a Virtual Filesystem. Obfuscated, convert each character as (255 - x). |
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
    root: VirtualFolder,
    version: Version,
    scale_factor: u32,
    timestamp: u32,
}

impl Multifile {
    const CURRENT_VERSION: Version = Version { major: 1, minor: 1 };
    const MAGIC: [u8; 6] = *b"pmf\0\n\r";

    /// Parses a `Panda3D` Multifile pre-header, which allows for comment lines starting with '#'.
    ///
    /// Returns either a [String] containing the header comment data, or an [`IOError`](Error::Io)
    /// if it reaches EOF before finding the Multifile magic ("pmf\0\n\r").
    fn parse_header_prefix(input: &mut DataCursor) -> Result<String> {
        let mut header_prefix = String::new();
        let mut line = String::new();

        loop {
            if input.read_u8()? == b'#' {
                let len = input.read_line(&mut line)?;
                if len == 0 {
                    return Err(Error::EndOfFile);
                }

                header_prefix.push('#');
                header_prefix.push_str(&line);

                line.clear();
            } else {
                input.set_position(input.position() - 1);
                return Ok(header_prefix);
            }
        }
    }

    pub fn from_path<P: AsRef<Path>>(path: P, offset: usize) -> Result<Self> {
        // Acquire file data
        let mut data = DataCursor::from_path(path, Endian::Little)?;
        data.set_position(offset);

        // Check if there's any pre-header
        let header_text = Self::parse_header_prefix(&mut data)?;
        if !header_text.is_empty() {
            log::info!("Multifile Pre-Header:\n{}\n", header_text);
        }

        // Parse Multifile data
        let mut multifile = Self::default();
        multifile.read_index(&mut data)?;

        // Print the actual filesystem out to debug
        log::debug!("{}", multifile.root);

        Ok(multifile)
    }

    pub fn read_index(&mut self, input: &mut DataCursor) -> Result<()> {
        log::trace!("Reading Index, Offset {:#X}", input.position());

        // Parse the Multifile header
        let mut magic = [0u8; 6];
        input.read_exact(&mut magic)?;

        if magic != Self::MAGIC {
            let error = Error::InvalidMagic {
                expected: format!("{:?}", from_utf8(&Self::MAGIC)?).into(),
            };
            log::error!("{}", error);
            return Err(error);
        }

        // Latest version is v1.1
        self.version = Version {
            major: input.read_i16()?,
            minor: input.read_i16()?,
        };

        if self.version != Self::CURRENT_VERSION {
            let error = Error::UnknownVersion {
                expected: Self::CURRENT_VERSION.to_string().into(),
            };
            log::error!("{}", error);
            return Err(error);
        }

        log::info!("Multifile v{}", self.version);

        // Only print scale factor if it's bigger than 1 since that's the norm
        self.scale_factor = input.read_u32()?;
        if self.scale_factor > 1 {
            log::info!("Scale Factor: {}", self.scale_factor);
        }

        // Timestamp added in v1.1
        if self.version.minor >= 1 {
            self.timestamp = input.read_u32()?;
            log::info!(
                "Last Modified: {} ({})",
                time::format_timestamp(self.timestamp.into())?,
                self.timestamp
            );
        }

        log::trace!("Starting Subfile Parse!");
        // Loop through each Subfile, using next_index as a linked list
        let mut next_index = input.read_u32()? * self.scale_factor;
        while next_index != 0 {
            log::trace!(
                "Reading Subfile Header at Offset {:#X}, Next Index at {:#X}",
                input.position(),
                next_index
            );
            let (mut subfile, filename) = Subfile::from_data(input, self)?;

            if subfile.flags.contains(SubfileFlags::Signature) {
                subfile.parse_signature()?;
            } else {
                self.root.create_file(filename.split('/').peekable(), subfile);
            }

            input.set_position(next_index as usize);
            next_index = input.read_u32()? * self.scale_factor;
        }
        Ok(())
    }

    //pub fn extract<P: AsRef<Path>>(&self, output: P) {}
}

impl Default for Multifile {
    fn default() -> Self {
        Self {
            root: VirtualFolder::default(),
            version: Self::CURRENT_VERSION,
            scale_factor: 1,
            timestamp: 0,
        }
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Default)]
    struct SubfileFlags: u16 {
        const Deleted = 1 << 0;
        const IndexInvalid = 1 << 1;
        const DataInvalid = 1 << 2;
        const Compressed = 1 << 3;
        const Encrypted = 1 << 4;
        const Signature = 1 << 5;
        const Text = 1 << 6;
    }
}

impl core::fmt::Display for SubfileFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        if self.is_empty() {
            write!(f, "")
        } else {
            write!(f, "{}", &self.0)
        }
    }
}

#[derive(Default)]
struct Subfile {
    data: DataCursor,
    timestamp: u32,
    flags: SubfileFlags,
}

impl Subfile {
    pub fn from_data(
        data: &mut DataCursor,
        multifile: &Multifile,
    ) -> Result<(Self, CompactString)> {
        let mut subfile = Self::default();
        let filename = subfile.read_index(data, multifile)?;
        Ok((subfile, filename))
    }

    /// This function takes a [`DataCursor`] to read data from and the parent [`Multifile`] for
    /// global settings, and populates the `Subfile` data.
    pub fn read_index(
        &mut self,
        input: &mut DataCursor,
        multifile: &Multifile,
    ) -> Result<CompactString> {
        let offset = input.read_u32()? * multifile.scale_factor;
        let data_length = input.read_u32()?;
        self.flags = SubfileFlags::from_bits_truncate(input.read_u16()?);

        let length;
        if self.flags.intersects(SubfileFlags::Compressed | SubfileFlags::Encrypted) {
            length = input.read_u32()?;
            log::debug!("Subfile is compressed or encrypted! Original length: {length}");
        } else {
            length = data_length;
        }

        self.timestamp = input.read_u32()?;
        if self.timestamp == 0 {
            self.timestamp = multifile.timestamp;
        }

        let name_length = input.read_u16()?;
        let mut filename = CompactString::default();
        filename.reserve(name_length.into());

        for _ in 0..name_length {
            filename.push((255 - input.read_u8()?) as char);
        }

        log::info!(
            "Offset: {offset:>#8X} | Length: {data_length:>#8X}{}{}",
            if filename.is_empty() {
                String::new()
            } else {
                format!(" | Filename: {filename}")
            },
            if self.flags.is_empty() {
                String::new()
            } else {
                format!(" | Subfile flags: {}", self.flags)
            }
        );

        self.read_data(input, offset as usize, length as usize)?;

        Ok(filename)
    }

    pub fn read_data(
        &mut self,
        input: &mut DataCursor,
        offset: usize,
        length: usize,
    ) -> Result<()> {
        log::trace!("Reading data! Offset {:#X}, Length {:#X}", offset, length);
        input.set_position(offset);

        self.data = DataCursor::new(vec![0u8; length], Endian::Little);
        input.read_exact(self.data.as_mut())?;
        Ok(())
    }

    pub fn parse_signature(&mut self) -> Result<Vec<Certificate>> {
        // Signature begins with u32 length, u8[] data
        let signature_length = self.data.read_u32()?;
        self.data.set_position(self.data.position() + signature_length as usize);

        // Next is number of certificates, followed by raw data blob
        let num_certificates = self.data.read_u32()?;

        let data_blob = self.data.remaining_slice().to_vec();
        let mut certificates = Vec::new();
        let mut offset = 0;

        for certificate_number in 1..=num_certificates {
            let certificate = Certificate::from_der(&data_blob[offset..])?;

            log::debug!(
                "Certificate {}\n{}",
                certificate_number,
                print_x509_info(certificate.cert())?
            );

            offset += certificate.len();
            certificates.push(certificate);
        }

        self.data.set_position(0);
        Ok(certificates)
    }
}
