use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomPrimitive {
    pub shade_model: ShadeModel,
    pub first_vertex: u32,
    pub num_vertices: u32,
    pub index_type: NumericType,
    pub usage_hint: UsageHint,
    /// Reference to the Vertex Data
    pub vertices_ref: Option<u32>,
    /// Pointer To Array of the Vertex Ends (only used if this is an abnormal primitive like a tristrip)
    pub ends_ref: Option<u32>,
}

impl GeomPrimitive {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        //cycler data
        let shade_model = ShadeModel::from(data.read_u8()?);
        let first_vertex = data.read_u32()?;
        let num_vertices = data.read_u32()?;
        let index_type = NumericType::from(data.read_u8()?);
        let usage_hint = UsageHint::from(data.read_u8()?);
        let vertices_ref = loader.read_pointer(data)?;

        let ends_ref = match loader.read_pta_id(data)? {
            0 => {
                // If the pointer is zero, that means that it's a NULL, so just store None. We still need to
                // read an array, but it's empty
                assert!(data.read_u32()? == 0);
                None
            }
            x if x >= loader.arrays.len() as u32 => {
                // We've found a new array! Let's read it and store it in the loader data, and then return the
                // new pointer.
                let size = data.read_u32()?;
                let mut array = Vec::with_capacity(size as usize);
                for _ in 0..size {
                    array.push(data.read_u32()?);
                }
                loader.arrays.push(array);
                Some(x)
            }
            x => {
                // We've already seen this array, just store the pointer and call it a day
                Some(x)
            }
        };

        Ok(Self {
            shade_model,
            first_vertex,
            num_vertices,
            index_type,
            usage_hint,
            vertices_ref,
            ends_ref,
        })
    }
}
