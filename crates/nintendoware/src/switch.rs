use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::path::Path;

use bitflags::bitflags;
use num_enum::FromPrimitive;
use orthrus_core::prelude::*;
use snafu::prelude::*;

use crate::error::*;

trait Read {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self>
    where
        Self: Sized;
}

struct Identifier;

#[rustfmt::skip]
impl Identifier {
    const STRING: u16 = 0x1F01;

    const SOUND_INFO_SECTION: u16 = 0x2100;
    const BANK_INFO_SECTION: u16 = 0x2101;
    const PLAYER_INFO_SECTION: u16 = 0x2102;
    const WAVE_ARCHIVE_INFO_SECTION: u16 = 0x2103;
    const SOUND_GROUP_INFO_SECTION: u16 = 0x2104;
    const GROUP_INFO_SECTION: u16 = 0x2105;
    const FILE_INFO_SECTION: u16 = 0x2106;

    const SOUND_INFO: u16 = 0x2200;
    const STREAM_SOUND_INFO: u16 = 0x2201;
    const WAVE_SOUND_INFO: u16 = 0x2202;
    const SEQUENCE_SOUND_INFO: u16 = 0x2203;

    const SOUND_ARCHIVE_PLAYER_INFO: u16 = 0x220B;

    const STREAM_TRACK_INFO: u16 = 0x220E;

    const STRING_TABLE: u16 = 0x2400;
    const PATRICIA_TREE: u16 = 0x2401;
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteOrderMark(u16);

#[allow(non_upper_case_globals)]
impl ByteOrderMark {
    pub const Big: ByteOrderMark = ByteOrderMark(0xFEFF);
    pub const Little: ByteOrderMark = ByteOrderMark(0xFFFE);
}

impl Default for ByteOrderMark {
    #[cfg(target_endian = "little")]
    #[inline]
    fn default() -> Self {
        ByteOrderMark::Little
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn default() -> Self {
        ByteOrderMark::Big
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl Read for Version {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut version = Self::default();
        version.major = data.read_u8()?;
        version.minor = data.read_u8()?;
        version.patch = data.read_u8()?;
        //This should always be zero, but I'm not going to enforce an assert here
        let _align = data.read_u8()?;
        Ok(version)
    }
}

impl core::fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct BinaryHeader {
    magic: [u8; 4],
    byte_order: ByteOrderMark,
    size: u16,
    version: Version,
    file_size: u32,
    num_sections: u16,
    //padding: [u8; 2]
}

impl Read for BinaryHeader {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Create a header, so we can copy in its magic
        let mut header = Self::default();

        // Read in the magic
        data.read_length(&mut header.magic)?;

        // Read the Byte Order Mark and use it to update our endianness
        header.byte_order = ByteOrderMark(data.read_u16()?);
        let endian = match header.byte_order {
            ByteOrderMark::Little => Endian::Little,
            ByteOrderMark::Big => Endian::Big,
            _ => InvalidDataSnafu { reason: "Invalid Byte Order Mark!" }.fail()?,
        };
        data.set_endian(endian);

        //Read the rest of the data
        header.size = data.read_u16()?;
        header.version = Version::read(data)?;
        header.file_size = data.read_u32()?;
        header.num_sections = data.read_u16()?;
        data.seek(SeekFrom::Current(2))?; //Skip alignment

        Ok(header)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct SizedReference {
    identifier: u16,
    //padding: [u8; 2]
    offset: u32,
    size: u32,
}

impl Read for SizedReference {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let identifier = data.read_u16()?;
        data.seek(SeekFrom::Current(2))?;

        let offset = data.read_u32()?;
        let size = data.read_u32()?;

        Ok(Self { identifier, offset, size })
    }
}

#[derive(Default, Debug)]
struct Reference {
    identifier: u16,
    //padding: [u8; 2]
    offset: u32,
}

impl Read for Reference {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let identifier = data.read_u16()?;
        data.seek(SeekFrom::Current(2))?;

        let offset = data.read_u32()?;

        Ok(Self { identifier, offset })
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct SectionHeader {
    magic: [u8; 4],
    size: u32,
}

impl Read for SectionHeader {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut header = SectionHeader::default();
        data.read_length(&mut header.magic)?;
        header.size = data.read_u32()?;
        Ok(header)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct Table<V: Read> {
    _marker: PhantomData<V>,
}

impl<V: Read> Table<V> {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Vec<V>> {
        let count = data.read_u32()?;

        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            values.push(V::read(data)?);
        }

        Ok(values)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct PatriciaNode {
    flags: u16,
    search_index: u16,
    left_index: u32,
    right_index: u32,
    string_id: u32,
    item_id: u32,
}

impl Read for PatriciaNode {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        Ok(Self {
            flags: data.read_u16()?,
            search_index: data.read_u16()?,
            left_index: data.read_u32()?,
            right_index: data.read_u32()?,
            string_id: data.read_u32()?,
            item_id: data.read_u32()?,
        })
    }
}

impl Default for PatriciaNode {
    fn default() -> Self {
        Self {
            flags: 0,
            search_index: 0xFFFF,
            left_index: 0xFFFFFFFF,
            right_index: 0xFFFFFFFF,
            string_id: 0xFFFFFFFF,
            item_id: 0xFFFFFFFF,
        }
    }
}

#[derive(Default, Debug)]
struct PatriciaTree {
    root_index: u32,
    nodes: Vec<PatriciaNode>,
}

impl PatriciaTree {
    fn get_node(&self, string: String) -> Result<&PatriciaNode> {
        let mut node = self.nodes.get(self.root_index as usize).ok_or(Error::NodeNotFound)?;
        let bytes = string.as_bytes();

        // Loop as long as we haven't hit a leaf node
        while (node.flags & 1) == 0 {
            // Separate out the string position and the bit location
            let pos = (node.search_index >> 3) as usize;
            let bit = (node.search_index & 7) as usize;

            let node_index;
            if (bytes[pos] & (1 << (7 - bit))) == 1 {
                node_index = node.right_index as usize;
            } else {
                node_index = node.left_index as usize;
            }
            node = self.nodes.get(node_index).ok_or(Error::NodeNotFound)?;
        }

        Ok(node)
    }
}

impl Read for PatriciaTree {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut tree = Self::default();

        // First, get the root index
        tree.root_index = data.read_u32()?;

        // Then, we can load in the node table
        tree.nodes = Table::read(data)?;

        Ok(tree)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct SendValue {
    main_send: u8,
    fx_send: [u8; 3],
}

impl Read for SendValue {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        Ok(Self {
            main_send: data.read_u8()?,
            fx_send: [data.read_u8()?, data.read_u8()?, data.read_u8()?],
        })
    }
}

#[derive(Debug, Default)]
struct StreamSoundExtension {
    stream_type_info: u32,
    loop_start_frame: u32,
    loop_end_frame: u32,
}

impl Read for StreamSoundExtension {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        Ok(Self {
            stream_type_info: data.read_u32()?,
            loop_start_frame: data.read_u32()?,
            loop_end_frame: data.read_u32()?,
        })
    }
}

#[derive(Debug, Default)]
struct StreamTrackInfo {
    volume: u8,
    pan: u8,
    span: u8,
    flags: u8,
    lpf_freq: u8,
    biquad_type: u8,
    biquad_value: u8,
    //padding: [u8; 1]
    global_channel_indices: Vec<u8>,
    send_value: SendValue,
}

impl Read for StreamTrackInfo {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Save our relative position
        let offset = data.position();

        let mut info = Self::default();
        info.volume = data.read_u8()?;
        info.pan = data.read_u8()?;
        info.span = data.read_u8()?;
        info.flags = data.read_u8()?;

        let global_channel_ref = Reference::read(data)?;
        let send_value_ref = Reference::read(data)?;

        info.lpf_freq = data.read_u8()?;
        info.biquad_type = data.read_u8()?;
        info.biquad_value = data.read_u8()?;
        data.seek(SeekFrom::Current(1))?;

        data.set_position(offset + global_channel_ref.offset as usize);
        // This is a raw type so I just do this manually instead of calling Table::read
        let index_count = data.read_u32()?;
        info.global_channel_indices = Vec::with_capacity(index_count as usize);
        for _ in 0..index_count {
            info.global_channel_indices.push(data.read_u8()?);
        }

        // Now we need to align, and theoretically that's where send_value is
        data.set_position((data.position() + 3) & !3);

        data.set_position(offset + send_value_ref.offset as usize);
        info.send_value = SendValue::read(data)?;

        Ok(info)
    }
}

#[derive(Debug, Default)]
struct StreamSoundInfo {
    valid_tracks: u16,
    channel_count: u16,
    pitch: f32,
    prefetch_id: u32,
    tracks: Vec<StreamTrackInfo>,
    send_value: SendValue,
    extension: StreamSoundExtension,
}

impl Read for StreamSoundInfo {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Save relative position
        let offset = data.position();

        let mut info = Self::default();

        info.valid_tracks = data.read_u16()?;
        info.channel_count = data.read_u16()?;

        // Reference to TrackInfoTable
        let track_info_ref = Reference::read(data)?;
        info.pitch = data.read_f32()?;

        let send_value_ref = Reference::read(data)?;
        let extension_ref = Reference::read(data)?;

        info.prefetch_id = data.read_u32()?;

        // Get the TrackInfo, which is a reference table to a bunch of StreamTrackInfos
        let track_table: Vec<Reference> = Table::read(data)?;

        data.set_position(offset + track_info_ref.offset as usize);
        // Pre-allocate and read all tracks in
        let mut tracks = Vec::with_capacity(track_table.len());
        for reference in &track_table {
            match reference.identifier {
                Identifier::STREAM_TRACK_INFO => {
                    tracks.push(StreamTrackInfo::read(data)?);
                }
                _ => InvalidDataSnafu { reason: "Unexpected Track Info Reference!" }.fail()?,
            }
        }

        info.tracks = tracks;

        data.set_position(offset + send_value_ref.offset as usize);
        info.send_value = SendValue::read(data)?;

        data.set_position(offset + extension_ref.offset as usize);
        info.extension = StreamSoundExtension::read(data)?;

        Ok(info)
    }
}

#[derive(Debug, Default)]
enum SoundDetails {
    Stream(StreamSoundInfo),
    Wave,
    Sequence,
    #[default]
    None,
}

bitflags! {
    struct OptionFlags: u32 {
        const StringId = 1 << 0;
        const PanParams = 1 << 1;
        const PlayerParams = 1 << 2;
        const SinglePlayParams = 1 << 3;

        const Sound3DOffset = 1 << 8;

        const RVLParamOffset = 1 << 16;
        const CTRParamOffset = 1 << 17;
        const CAFEParamOffset = 1 << 18;

        const UserParam = 1 << 31;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum PanMode {
    #[default]
    /// Treat stereo as two separate tracks
    Dual,
    /// Treat stereo as one balanced track
    Balance,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum PanCurve {
    #[default]
    /// Square Root Curve, -3 dB center, 0 dB ends
    Sqrt,
    /// Square Root Curve, 0 dB center, 3 dB ends
    Sqrt0,
    /// Square Root Curve, 0 dB center, 0dB ends
    Sqrt0Clamp,
    /// Sin/Cos Curve, -3 dB center, 0 dB ends
    SinCos,
    /// Sin/Cos Curve, 0 dB center, 3 dB ends
    SinCos0,
    /// Sin/Cos Curve, 0 dB center, 0 dB ends
    SinCos0Clamp,
    /// Linear Curve, -6 dB center, 0 dB ends
    Linear,
    /// Linear Curve, 0 dB center, 6 dB ends
    Linear0,
    /// Linear Curve, 0 dB center, 0 dB ends
    Linear0Clamp,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
enum PlayType {
    #[default]
    None,
    Oldest,
    OldestDuration,
    Newest,
    NewestDuration,
}

bitflags! {
    #[derive(Default, Debug)]
    pub struct Sound3DFlags: u32 {
        const Volume = 1 << 0;
        const Priority = 1 << 1;
        const Pan = 1 << 2;
        const SurroundPan = 1 << 3;
        const Filter = 1 << 4;
    }
}

#[derive(Default, Debug)]
struct Sound3DInfo {
    flags: Sound3DFlags,
    decay_ratio: f32,
    decay_curve: u8,
    doppler_factor: u8,
    //padding: [u8; 2]
    options: u32,
}

impl Read for Sound3DInfo {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let mut info = Self::default();

        info.flags = Sound3DFlags::from_bits_truncate(data.read_u32()?);
        info.decay_ratio = data.read_f32()?;
        info.decay_curve = data.read_u8()?;
        info.doppler_factor = data.read_u8()?;
        data.seek(SeekFrom::Current(2))?;
        info.options = data.read_u32()?;

        Ok(info)
    }
}

#[derive(Debug)]
struct SoundInfo {
    file_id: u32,
    player_id: u32,
    volume: u8,
    filter: u8,
    //padding: [u8; 2]
    //SoundDetailsRef
    options: u32,

    string_id: u32,
    pan_mode: PanMode,
    pan_curve: PanCurve,
    player_prio: u8,
    player_actor_id: u8,
    play_type: PlayType,
    play_duration: u16,
    is_front_bypass: bool,
    user_param: u32,
    virtual_info: Sound3DInfo,
    details: SoundDetails,
}

impl Default for SoundInfo {
    fn default() -> Self {
        Self {
            file_id: 0,
            player_id: 0,
            volume: 0,
            filter: 0,
            details: SoundDetails::None,
            options: 0,

            string_id: 0xFFFFFFFF,
            pan_mode: PanMode::default(),
            pan_curve: PanCurve::default(),
            player_prio: 64,
            player_actor_id: 0,
            play_type: PlayType::default(),
            play_duration: 0xFFFF,
            is_front_bypass: false,
            user_param: 0xFFFFFFFF,
            virtual_info: Sound3DInfo::default(),
        }
    }
}

impl SoundInfo {
    /// Checks if a specific bit is set, based on bit position (0-31)
    fn get_value(&self, bit_pos: u32) -> Option<usize> {
        // Check that we didn't accidentally go past bounds
        if bit_pos > 31 {
            return None;
        }

        // Check if the bit is even set
        if (self.options & (1 << bit_pos)) == 0 {
            return None;
        }

        // If it is, count the number of bits set before it (i.e. what set bit is it)
        let mut count: usize = 0;
        for n in 0..bit_pos {
            if (self.options & (1 << n)) != 0 {
                count += 1;
            }
        }

        Some(count * core::mem::size_of::<u32>())
    }

    fn read_string_id<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<u32> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(0) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.string_id = value;
        }

        Some(self.string_id)
    }

    fn read_pan_mode<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<PanMode> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(1) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.pan_mode = PanMode::from((value & 0xFF) as u8);
        }

        Some(self.pan_mode)
    }

    fn read_pan_curve<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<PanCurve> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(1) {
            data.set_position(offset + position as usize);

            let value = data.read_u32().ok().unwrap();
            self.pan_curve = PanCurve::from(((value >> 8) & 0xFF) as u8);
        }

        Some(self.pan_curve)
    }

    fn read_player_prio<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<u8> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(2) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.player_prio = (value & 0xFF) as u8;
        }

        Some(self.player_prio)
    }

    fn read_player_actor_id<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<u8> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(2) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.player_actor_id = ((value >> 8) & 0xFF) as u8;
        }

        Some(self.player_actor_id)
    }

    fn read_play_type<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<PlayType> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(3) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.play_type = PlayType::from((value & 0xFF) as u8);
        }

        Some(self.play_type)
    }

    fn read_play_duration<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<u16> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(3) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.play_duration = ((value >> 16) & 0xFFFF) as u16;
        }

        Some(self.play_duration)
    }

    /// Returns an offset to Sound3DInfo parameters.
    fn get_3d_info_offset<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<u32> {
        let mut value = None;
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(8) {
            data.set_position(offset + position);

            value = data.read_u32().ok();
        }

        value
    }

    fn is_front_bypass<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<bool> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(17) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.is_front_bypass = (value & 1) == 1;
        }

        Some(self.is_front_bypass)
    }

    fn read_user_param<T: DataCursorTrait + EndianRead>(
        &mut self, data: &mut T, position: usize,
    ) -> Option<u32> {
        // If the bit is set, get its data
        if let Some(offset) = self.get_value(31) {
            data.set_position(offset + position);

            let value = data.read_u32().ok().unwrap();
            self.user_param = value;
        }

        Some(self.user_param)
    }
}

impl Read for SoundInfo {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        let readback = data.position();

        let file_id = data.read_u32()?;
        let player_id = data.read_u32()?;
        let volume = data.read_u8()?;
        let filter = data.read_u8()?;
        data.seek(SeekFrom::Current(2))?;

        // Reference to SoundDetails
        let details_ref = Reference::read(data)?;
        let options = data.read_u32()?;

        let mut info = Self {
            file_id,
            player_id,
            volume,
            filter,
            options,
            ..Default::default()
        };

        let position = data.position();

        info.read_string_id(data, position);
        info.read_pan_mode(data, position);
        info.read_pan_curve(data, position);
        info.read_player_prio(data, position);
        info.read_player_actor_id(data, position);
        info.read_play_type(data, position);
        info.read_play_duration(data, position);

        if let Some(offset) = info.get_3d_info_offset(data, position) {
            data.set_position(readback + offset as usize);
            info.virtual_info = Sound3DInfo::read(data)?;
        }

        info.is_front_bypass(data, position);
        info.read_user_param(data, position);

        data.set_position(readback + details_ref.offset as usize);
        info.details = match details_ref.identifier {
            Identifier::STREAM_SOUND_INFO => SoundDetails::Stream(StreamSoundInfo::read(data)?),
            Identifier::WAVE_SOUND_INFO => SoundDetails::Wave,
            Identifier::SEQUENCE_SOUND_INFO => SoundDetails::Sequence,
            _ => SoundDetails::None,
        };

        Ok(info)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct StringBlock {
    table: Vec<String>,
    tree: PatriciaTree,
}

impl StringBlock {
    /// Unique identifier that tells us if we're reading a String Block.
    pub const MAGIC: [u8; 4] = *b"STRG";

    fn read_string_table<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Vec<String>> {
        // Store relative position
        let offset = data.position();

        // Read in the reference table
        let references: Vec<SizedReference> = Table::read(data)?;

        // Then we can process all strings, pre-allocate since we know the count ahead of time
        let mut strings = Vec::with_capacity(references.len() as usize);
        for reference in &references {
            match reference.identifier {
                Identifier::STRING => {
                    // Go to that position in the string blob
                    data.set_position(offset + reference.offset as usize);

                    // Read the string and store it, includes the trailing \0
                    let string = data.get_slice(reference.size as usize)?.to_vec();
                    strings.push(String::from_utf8(string).map_err(|_| data::Error::InvalidUtf8)?);
                }
                _ => InvalidDataSnafu { reason: "Unexpected String Identifier!" }.fail()?,
            }
        }

        Ok(strings)
    }
}

impl Read for StringBlock {
    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Store relative position
        let offset = data.position();

        // Read both sections
        let mut sections: [Reference; 2] = Default::default();

        for section in &mut sections {
            *section = Reference::read(data)?;
        }

        // Then process each section
        let mut strings = Self::default();

        for section in &mut sections {
            data.set_position(offset + section.offset as usize);
            match section.identifier {
                Identifier::STRING_TABLE => {
                    strings.table = Self::read_string_table(data)?;
                }
                Identifier::PATRICIA_TREE => {
                    strings.tree = PatriciaTree::read(data)?;
                }
                _ => InvalidDataSnafu { reason: "Unexpected String Block Identifier!" }.fail()?,
            }
        }

        Ok(strings)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct InfoBlock {
    sounds: Vec<SoundInfo>,
}

impl InfoBlock {
    /// Unique identifier that tells us if we're reading an Info Block.
    pub const MAGIC: [u8; 4] = *b"INFO";

    fn read<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<Self> {
        // Store relative position
        let offset = data.position();

        let mut info = Self::default();

        // Read all references
        let mut sections: [Reference; 8] = Default::default();
        for section in &mut sections {
            *section = Reference::read(data)?;
        }

        for section in &mut sections {
            data.set_position(offset + section.offset as usize);
            match section.identifier {
                Identifier::SOUND_INFO_SECTION => {
                    // Sound Info
                    // Load the reference table
                    let references: Vec<Reference> = Table::read(data)?;

                    // Pre-allocate the array with the number of entries
                    info.sounds = Vec::with_capacity(references.len());

                    for reference in &references {
                        match reference.identifier {
                            Identifier::SOUND_INFO => {
                                data.set_position(
                                    offset + (section.offset + reference.offset) as usize,
                                );
                                let sound_info = SoundInfo::read(data)?;
                                info.sounds.push(sound_info);
                            }
                            _ => InvalidDataSnafu { reason: "Unexpected Sound Info Identifier!" }
                                .fail()?,
                        }
                    }
                }
                Identifier::BANK_INFO_SECTION => {}
                Identifier::PLAYER_INFO_SECTION => {}
                Identifier::WAVE_ARCHIVE_INFO_SECTION => {}
                Identifier::SOUND_GROUP_INFO_SECTION => {}
                Identifier::GROUP_INFO_SECTION => {}
                Identifier::FILE_INFO_SECTION => {}
                Identifier::SOUND_ARCHIVE_PLAYER_INFO => {}
                _ => InvalidDataSnafu { reason: "Unexpected Info Section Identifier!" }.fail()?,
            }
        }

        Ok(info)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct FileBlock {
    header: SectionHeader,
}

impl FileBlock {
    /// Unique identifier that tells us if we're reading a File Block.
    pub const MAGIC: [u8; 4] = *b"FILE";
}

//-------------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
/// Binary caFe Sound ARchive
pub struct BFSAR {
    header: BinaryHeader,
    strings: StringBlock,
    info: InfoBlock,
    files: FileBlock,
}

impl BFSAR {
    /// Unique identifier that tells us if we're reading a Sound Archive.
    pub const MAGIC: [u8; 4] = *b"FSAR";

    #[inline]
    #[allow(dead_code)]
    fn read_header<T: DataCursorTrait + EndianRead>(data: &mut T) -> Result<BinaryHeader> {
        // Read the header
        let header = BinaryHeader::read(data)?;

        //Now we need to verify that it's what we actually expected
        ensure!(
            header.magic == Self::MAGIC,
            InvalidMagicSnafu { expected: Self::MAGIC }
        );

        ensure!(
            header.size == 0x40,
            InvalidDataSnafu { reason: "Header size must be 0x40!" }
        );

        ensure!(
            data.len() == header.file_size as usize,
            InvalidDataSnafu { reason: "Unexpected file size!" }
        );

        ensure!(
            header.num_sections == 3,
            InvalidDataSnafu { reason: "Unexpected section count!" }
        );

        Ok(header)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn open<P: AsRef<Path>>(input: P) -> Result<()> {
        let data = std::fs::read(input)?;
        Self::load(data)
    }

    #[inline]
    pub fn load<I: Into<Box<[u8]>>>(input: I) -> Result<()> {
        // Initialize the data
        let mut data = DataCursor::new(input, Endian::Big);

        // Start creating our return struct
        let mut archive = Self::default();

        // Read the file header
        archive.header = Self::read_header(&mut data)?;

        // Read the references to all sections
        let mut sections: [SizedReference; 3] = Default::default();
        for n in 0..sections.len() {
            sections[n] = SizedReference::read(&mut data)?;
        }

        // Align to a 32-byte boundary
        data.set_position((data.position() + 31) & !31);

        // Then read all the section data
        for section in &sections {
            data.set_position(section.offset as usize);

            let section = SectionHeader::read(&mut data)?;
            match section.magic {
                StringBlock::MAGIC => {
                    archive.strings = StringBlock::read(&mut data)?;
                }
                InfoBlock::MAGIC => {
                    archive.info = InfoBlock::read(&mut data)?;
                }
                FileBlock::MAGIC => {}
                _ => InvalidDataSnafu { reason: "Unexpected BFSAR Section!" }.fail()?,
            }
        }

        /*
        Good lord this took so long to be able to do
        for info in archive.info.sounds {
            match info.details {
                SoundDetails::Stream(stream) => {
                    let filename = archive.strings.table[info.string_id as usize].clone();
                    println!(
                        "{} has loop {} to {}", filename.split_at(filename.len() - 1).0,
                        stream.extension.loop_start_frame, stream.extension.loop_end_frame
                    );
                }
                _ => (),
            }
        }*/

        Ok(())
    }
}
