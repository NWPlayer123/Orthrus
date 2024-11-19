//! Adds support for the Audio Stream format used by NintendoWare for Revolution (NW4R).
//! 
//! # Revisions
//! **Version 0.1** Used for prerelease games (see Trauma Center: Second Opinion) **Version 1.0** Used by the
//! majority of NW4R games
//! 
//! # Format
//! The BRSTM format, much like the rest of the NintendoWare binary formats, consists of a [shared
//! header](super#shared-header), along with a number of "blocks" specific to each format.
//! 
//! 

use crate::error::*;

use orthrus_core::prelude::*;
use snafu::prelude::*;

use super::common::{BlockHeader, FileHeader};

#[cfg(feature = "std")]
use std::{fs::File, io::BufReader, path::Path};

//TODO: move to common?
#[derive(Debug)]
#[allow(dead_code)]
struct DataRef {
    //TODO: does it really matter to split this up?
    tag: u32,
    value: u32,
}

impl DataRef {
    #[inline]
    fn new<T: ReadExt>(data: &mut T) -> Result<Self> {
        Ok(Self { tag: data.read_u32()?, value: data.read_u32()? })
    }
}

#[derive(Debug)]
struct SectionInfo {
    offset: u32,
    size: u32,
}

impl SectionInfo {
    #[inline]
    fn new<T: ReadExt>(data: &mut T) -> Result<Self> {
        Ok(Self { offset: data.read_u32()?, size: data.read_u32()? })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct ExtendedHeader {
    file_header: FileHeader,
    head_block: SectionInfo,
    adpc_block: SectionInfo,
    data_block: SectionInfo,
}

pub struct StreamFile {}

impl StreamFile {
    /// Unique identifier that tells us if we're reading a BRSTM file.
    pub const MAGIC: [u8; 4] = *b"RSTM";
    /// Identifier for the ADPC section.
    pub const ADPC_MAGIC: [u8; 4] = *b"ADPC";
    /// Identifier for the DATA section.
    pub const DATA_MAGIC: [u8; 4] = *b"DATA";

    #[inline]
    fn read_header<T: ReadExt>(data: &mut T) -> Result<ExtendedHeader> {
        let file_header = FileHeader::new(data, Self::MAGIC)?;
        let head_block = SectionInfo::new(data)?;
        let adpc_block = SectionInfo::new(data)?;
        let data_block = SectionInfo::new(data)?;
        Ok(ExtendedHeader { file_header, head_block, adpc_block, data_block })
    }

    #[inline]
    #[cfg(feature = "std")]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = BufReader::new(File::open(path)?);
        Self::load(data)
    }

    #[inline]
    pub fn load<T: IntoDataStream>(input: T) -> Result<Self> {
        let mut data = input.into_stream(Endian::Big);

        // These formats work off "blocks" of data, so we need to know the position relative to the start of
        // the section.
        let position = data.position()?;
        let header = Self::read_header(&mut data)?;
        data.set_position(position + u64::from(header.file_header.header_size))?;

        let _head_block = head_block::HeadBlock::new(&mut data, &header.head_block)?;

        //ADPC only if ADPCM codec

        Ok(Self {})
    }
}

mod head_block {

    use super::*;

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Header {
        stream_info: DataRef,
        track_info: DataRef,
        channel_info: DataRef,
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    struct StreamInfo {
        codec: u8,
        loop_flag: u8,
        channel_count: u8,
        /// This is stored as a u24, allowing for a sample rate of up to 0xFFFFFF (16,777,215 Hz).
        sample_rate: u32,
        block_info_offset: u16,
        loop_start: u32,
        loop_end: u32,
        data_offset: u32,
        block_count: u32,
        block_size: u32,
        block_samples: u32,
        last_block_size: u32,
        last_block_samples: u32,
        last_block_size_align: u32,
        adpcm_data_interval: u32,
        adpcm_data_size: u32,
    }

    impl StreamInfo {
        #[inline]
        fn new<T: ReadExt>(data: &mut T) -> Result<Self> {
            let codec = data.read_u8()?;
            let loop_flag = data.read_u8()?;
            let channel_count = data.read_u8()?;
            let sample_rate_raw: [u8; 3] = data.read_exact::<3>()?;
            let sample_rate = match data.endian() {
                Endian::Big => {
                    u32::from_be_bytes([0, sample_rate_raw[0], sample_rate_raw[1], sample_rate_raw[2]])
                }
                Endian::Little => {
                    u32::from_le_bytes([sample_rate_raw[0], sample_rate_raw[1], sample_rate_raw[2], 0])
                }
            };
            let block_info_offset = data.read_u16()?;
            let loop_start = data.read_u32()?;
            let loop_end = data.read_u32()?;
            let data_offset = data.read_u32()?;
            let block_count = data.read_u32()?;
            let block_size = data.read_u32()?;
            let block_samples = data.read_u32()?;
            let last_block_size = data.read_u32()?;
            let last_block_samples = data.read_u32()?;
            let last_block_size_align = data.read_u32()?;
            let adpcm_data_interval = data.read_u32()?;
            let adpcm_data_size = data.read_u32()?;

            Ok(Self {
                codec,
                loop_flag,
                channel_count,
                sample_rate,
                block_info_offset,
                loop_start,
                loop_end,
                data_offset,
                block_count,
                block_size,
                block_samples,
                last_block_size,
                last_block_samples,
                last_block_size_align,
                adpcm_data_interval,
                adpcm_data_size,
            })
        }
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    struct TrackTable {
        metadata: Vec<TrackInfoEx>,
    }

    // This is the extended variant, anything with track type 0 gets converted to this
    #[derive(Debug)]
    #[allow(dead_code)]
    struct TrackInfoEx {
        volume: u8,
        pan: u8,
        channels: Vec<u8>,
    }

    impl TrackTable {
        fn new<T: ReadExt + SeekExt>(data: &mut T, start_position: u64) -> Result<Self> {
            // Read all metadata
            let track_count = data.read_u8()?;
            let track_type = data.read_u8()?;
            data.read_u16()?; //padding

            // Now we need to create a list of all our tracks
            let mut refs = Vec::with_capacity(track_count.into());
            for _ in 0..track_count {
                refs.push(DataRef::new(data)?);
            }

            // For each track, we need to read its data
            let mut metadata = Vec::with_capacity(track_count.into());
            for data_ref in &refs {
                // This will allow for alignment when we have even-numbered channel counts
                data.set_position(start_position + u64::from(data_ref.value))?;

                metadata.push(match track_type {
                    0 => {
                        // TrackInfo
                        let volume = 127;
                        let pan = 64;
                        let channel_count = core::cmp::min(data.read_u8()?, 32);
                        let channels = data.read_slice(channel_count.into())?.into_owned();
                        TrackInfoEx { volume, pan, channels }
                    }
                    1 => {
                        // TrackInfoEx
                        let volume = data.read_u8()?;
                        let pan = data.read_u8()?;
                        data.read_u16()?; //padding
                        data.read_u32()?; //reserved
                        let channel_count = data.read_u8()?;
                        let channels = data.read_slice(channel_count.into())?.into_owned();
                        TrackInfoEx { volume, pan, channels }
                    }
                    _ => InvalidDataSnafu {
                        position: start_position + u64::from(data_ref.value),
                        reason: "Invalid Track Type",
                    }
                    .fail()?,
                });
            }

            Ok(Self { metadata })
        }
    }

    struct ChannelInfo {}

    impl ChannelInfo {
        fn new<T: ReadExt>(data: &mut T) -> Result<Self> {
            Ok(Self {})
        }
    }

    struct ChannelTable {
        channels: Vec<ChannelInfo>,
    }

    impl ChannelTable {
        fn new<T: ReadExt + SeekExt>(data: &mut T, start_position: u64) -> Result<Self> {
            let channel_count = data.read_u8()?;
            data.read_exact::<3>()?; //padding
            let mut channels = Vec::with_capacity(channel_count.into());
            for _ in 0..channel_count {
                channels.push(ChannelInfo::new(data)?);
            }
            Ok(Self { channels })
        }
    }

    #[allow(dead_code)]
    pub(super) struct HeadBlock {
        stream_info: StreamInfo,
        track_table: TrackTable,
        channel_table: ChannelTable,
    }

    impl HeadBlock {
        /// Unique identifier that tells us we're reading a HEAD section.
        pub const MAGIC: [u8; 4] = *b"HEAD";

        #[inline]
        fn read_header<T: ReadExt>(data: &mut T) -> Result<Header> {
            let stream_info = DataRef::new(data)?;
            let track_info = DataRef::new(data)?;
            let channel_info = DataRef::new(data)?;
            Ok(Header { stream_info, track_info, channel_info })
        }

        #[inline]
        pub fn new<T: ReadExt + SeekExt>(data: &mut T, info: &SectionInfo) -> Result<Self> {
            // First, let's verify that the Block Header is what we expect
            let start_position = data.position()?;
            let block_header = BlockHeader::new(data, Self::MAGIC)?;
            ensure!(
                block_header.block_size == info.size,
                InvalidDataSnafu { position: start_position, reason: "Unexpected Block Section" }
            );
            ensure!(
                start_position == info.offset.into(),
                InvalidDataSnafu { position: start_position, reason: "Unexpected Block Alignment" }
            );

            // Now we're at the start of the actual HEAD section, which includes info for 3 sections
            let start_position = data.position()?;
            let header = Self::read_header(data)?;

            // Start of the Stream Info sub-block
            let position = data.position()?;
            ensure!(
                position - start_position == header.stream_info.value.into(),
                InvalidDataSnafu { position, reason: "Unexpected Sub-Block Encountered" }
            );
            let stream_info = StreamInfo::new(data)?;

            // Start of the Track Table sub-block
            let position = data.position()?;
            ensure!(
                position - start_position == header.track_info.value.into(),
                InvalidDataSnafu { position, reason: "Unexpected Sub-Block Encountered" }
            );
            let track_table = TrackTable::new(data, start_position)?;

            // Start of the Channel Table sub-block
            let position = data.position()?;
            ensure!(
                position - start_position == header.channel_info.value.into(),
                InvalidDataSnafu { position, reason: "Unexpected Sub-Block Encountered" }
            );
            let channel_table = ChannelTable::new(data, start_position)?;

            Ok(Self { stream_info, track_table, channel_table })
        }
    }
}
