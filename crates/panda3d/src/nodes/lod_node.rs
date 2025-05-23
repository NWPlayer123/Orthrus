use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct Switch {
    pub start: f32,
    pub end: f32,
}

impl Switch {
    #[inline]
    fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let start = data.read_float()?;
        let end = data.read_float()?;
        Ok(Switch { start, end })
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct LODNode {
    pub inner: PandaNode,
    pub center: Vec3,
    pub switch_vector: Vec<Switch>,
    pub lod_scale: f32,
}

impl Node for LODNode {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = PandaNode::create(loader, data)?;

        //Cycler data
        let center = Vec3::read(data)?;

        let num_switches = data.read_u16()?;
        let mut switch_vector = Vec::with_capacity(num_switches as usize);
        for _ in 0..num_switches {
            switch_vector.push(Switch::create(loader, data)?);
        }

        let lod_scale = 1.0;

        Ok(Self { inner, center, switch_vector, lod_scale })
    }
}

impl GraphDisplay for LODNode {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{LODNode|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(label, "|center: {}|", self.center)?;
        write!(label, "switches: [")?;
        let mut first = true;
        for switch in &self.switch_vector {
            if !first {
                write!(label, ", ")?;
            }
            write!(label, "{{start: {}|end: {}}}", switch.start, switch.end)?;
            first = false;
        }
        write!(label, "]|")?;
        write!(label, "lod_scale: {}", self.lod_scale)?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for LODNode {
    type Target = PandaNode;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for LODNode {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
