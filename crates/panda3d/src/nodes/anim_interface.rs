use super::prelude::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, FromPrimitive)]
#[repr(u8)]
pub(crate) enum PlayMode {
    #[default]
    Pose,
    Play,
    Loop,
    PingPong,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct AnimInterface {
    num_frames: u32,
    frame_rate: f32,
    play_mode: PlayMode,
    start_time: f32,
    start_frame: f32,
    play_frames: f32,
    from_frame: u32,
    to_frame: u32,
    play_rate: f32,
    effective_frame_rate: f32,
    paused: bool,
    paused_f: f32,
}

impl AnimInterface {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let num_frames = data.read_u32()?;

        // CData
        let frame_rate = data.read_float()?;
        let play_mode = PlayMode::from(data.read_u8()?);
        let start_time = data.read_float()?;
        let start_frame = data.read_float()?;
        let play_frames = data.read_float()?;
        let from_frame = data.read_u32()?;
        let to_frame = data.read_u32()?;
        let play_rate = data.read_float()?;
        let effective_frame_rate = frame_rate * play_rate;
        let paused = data.read_bool()?;
        let paused_f = data.read_float()?;

        Ok(Self {
            num_frames,
            frame_rate,
            play_mode,
            start_time,
            start_frame,
            play_frames,
            from_frame,
            to_frame,
            play_rate,
            effective_frame_rate,
            paused,
            paused_f,
        })
    }
}

impl GraphDisplay for AnimInterface {
    fn write_data(
            &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, is_root: bool,
        ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{AnimInterface|")?;
        }

        // Fields
        write!(label, "num_frames: {}|", self.num_frames)?;
        write!(label, "frame_rate: {}|", self.frame_rate)?;
        write!(label, "play_mode: {:#?}|", self.play_mode)?;
        write!(label, "start_time: {}|", self.start_time)?;
        write!(label, "start_frame: {}|", self.start_frame)?;
        write!(label, "play_frames: {}|", self.play_frames)?;
        write!(label, "from_frame: {}|", self.from_frame)?;
        write!(label, "play_rate: {}|", self.play_rate)?;
        write!(label, "effective_frame_rate: {}|", self.effective_frame_rate)?;
        write!(label, "paused: {}|", self.paused)?;
        write!(label, "paused_f: {}|", self.paused_f)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}