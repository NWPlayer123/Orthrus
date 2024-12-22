use super::prelude::*;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub(crate) enum CullMode {
    None,
    #[default]
    Clockwise,
    CounterClockwise,
    Unchanged,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CullFaceAttrib {
    pub mode: CullMode,
    pub reverse: bool,
}

impl CullFaceAttrib {
    #[inline]
    pub fn get_effective_mode(&self) -> CullMode {
        if self.reverse {
            match self.mode {
                CullMode::Clockwise | CullMode::Unchanged => CullMode::CounterClockwise,
                CullMode::CounterClockwise => CullMode::Clockwise,
                CullMode::None => CullMode::None,
            }
        } else {
            match self.mode {
                CullMode::Clockwise | CullMode::Unchanged => CullMode::Clockwise,
                CullMode::CounterClockwise => CullMode::CounterClockwise,
                CullMode::None => CullMode::None,
            }
        }
    }
}

impl Node for CullFaceAttrib {
    #[inline]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram<'_>) -> Result<Self, bam::Error> {
        let mode = CullMode::from(data.read_u8()?);
        let reverse = data.read_bool()?;
        Ok(Self { mode, reverse })
    }
}

impl GraphDisplay for CullFaceAttrib {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, _connections: &mut Vec<u32>, _is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        write!(label, "{{CullFaceAttrib|")?;

        // Fields
        write!(label, "mode: {:?}|", self.mode)?;
        write!(label, "reverse: {}", self.reverse)?;

        // Footer
        write!(label, "}}")?;
        Ok(())
    }
}
