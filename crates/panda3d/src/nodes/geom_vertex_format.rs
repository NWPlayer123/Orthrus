use super::{geom_vertex_anim_spec::GeomVertexAnimationSpec, prelude::*};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexFormat {
    animation: GeomVertexAnimationSpec,
    arrays: Vec<Option<u32>>,
}

impl GeomVertexFormat {
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let animation = GeomVertexAnimationSpec::create(loader, data)?;

        let mut format = Self {
            animation, ..Default::default()
        };

        let num_arrays = data.read_u16()?;
        for _ in 0..num_arrays {
            format.arrays.push(loader.read_pointer(data)?);
        }
        Ok(format)
    }
}
