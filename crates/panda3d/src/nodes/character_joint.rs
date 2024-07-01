use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CharacterJoint {
    pub matrix: MovingPartMatrix,
    pub character_ref: Option<u32>,
    pub net_node_refs: Vec<u32>,
    pub local_node_refs: Vec<u32>,
    pub initial_net_transform_inverse: Mat4,
}

impl CharacterJoint {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let matrix = MovingPartMatrix::create(loader, data)?;
        let character_ref = match loader.get_minor_version() >= 4 {
            true => loader.read_pointer(data)?,
            false => None,
        };

        let num_net_nodes = data.read_u16()?;
        let mut net_node_refs = Vec::with_capacity(num_net_nodes as usize);
        for _ in 0..num_net_nodes {
            net_node_refs.push(loader.read_pointer(data)?.unwrap());
        }

        let num_local_nodes = data.read_u16()?;
        let mut local_node_refs = Vec::with_capacity(num_local_nodes as usize);
        for _ in 0..num_local_nodes {
            local_node_refs.push(loader.read_pointer(data)?.unwrap());
        }

        let initial_net_transform_inverse = Mat4::read(data)?;

        Ok(Self {
            matrix,
            character_ref,
            net_node_refs,
            local_node_refs,
            initial_net_transform_inverse,
        })
    }
}
