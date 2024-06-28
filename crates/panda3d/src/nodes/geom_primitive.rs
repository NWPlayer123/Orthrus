use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomPrimitive {
    pub shade_model: ShadeModel,
    pub first_vertex: i32,
    pub num_vertices: i32,
    pub index_type: NumericType,
    pub usage_hint: UsageHint,
    pub vertices: u32,
    pub ptr_to_array: u32,
    pub array: Vec<u32>,
}

impl GeomPrimitive {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        //cycler data
        let shade_model = ShadeModel::from(data.read_u8()?);
        let first_vertex = data.read_i32()?;
        let num_vertices = data.read_i32()?;
        let index_type = NumericType::from(data.read_u8()?);
        let usage_hint = UsageHint::from(data.read_u8()?);
        let vertices = loader.read_pointer(data)?.unwrap();
        let ptr_to_array = loader.read_pta_id(data)?;
        let mut array: Vec<u32>;
        //TODO: clean this up once I have more data
        if ptr_to_array == 0 {
            unimplemented!("Not sure what to do with this");
        } else {
            let size = data.read_u32()?;
            array = Vec::with_capacity(size as usize);
            for _ in 0..size {
                array.push(data.read_u32()?);
            }
        }

        Ok(Self {
            shade_model,
            first_vertex,
            num_vertices,
            index_type,
            usage_hint,
            vertices,
            ptr_to_array,
            array,
        })
    }
}
