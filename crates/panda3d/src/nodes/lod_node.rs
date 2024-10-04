use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct Switch {
    start: f32,
    end: f32,
}

impl Switch {
    #[inline]
    pub fn create(_loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let start = data.read_float()?;
        let end = data.read_float()?;
        Ok(Switch { start, end })
    }
}

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct LODNode {
    node: PandaNode,
    center: Vec3,
    switch_vector: Vec<Switch>,
    lod_scale: f32,
}

impl LODNode {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let node = PandaNode::create(loader, data)?;

        //Cycler data
        let center = Vec3::read(data)?;

        let num_switches = data.read_u16()?;
        let mut switch_vector = Vec::with_capacity(num_switches as usize);
        for _ in 0..num_switches {
            switch_vector.push(Switch::create(loader, data)?);
        }

        let lod_scale = 1.0;

        Ok(Self { node, center, switch_vector, lod_scale })
    }
}
