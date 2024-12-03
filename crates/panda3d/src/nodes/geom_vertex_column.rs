use super::prelude::*;

pub const VERTEX_COLUMN_ALIGNMENT: u8 = 4;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct GeomVertexColumn {
    pub name_ref: u32,
    pub num_components: u8,
    pub numeric_type: NumericType,
    pub contents: Contents,
    pub start: u16,
    pub column_alignment: u8,

    pub num_elements: u8,
    pub element_stride: u16,
    pub num_values: u16,
    pub component_bytes: u8,
    pub total_bytes: u32,
}

impl GeomVertexColumn {
    #[inline]
    fn setup(&mut self, loader: &mut BinaryAsset) {
        self.num_values = self.num_components.into();

        if self.numeric_type == NumericType::StdFloat {
            match loader.header.use_double {
                true => NumericType::F64,
                false => NumericType::F32,
            };
        }

        match self.numeric_type {
            NumericType::U8 | NumericType::I8 => self.component_bytes = 1,
            NumericType::U16 | NumericType::I16 => self.component_bytes = 2,
            NumericType::U32 | NumericType::I32 => self.component_bytes = 4,
            NumericType::PackedDCBA | NumericType::PackedDABC => {
                self.component_bytes = 4;
                self.num_values *= 4;
            }
            NumericType::F32 => self.component_bytes = 4,
            NumericType::F64 => self.component_bytes = 8,
            NumericType::PackedUFloat => {
                self.component_bytes = 4;
                self.num_values *= 3;
            }
            NumericType::StdFloat => unreachable!("We disambiguate this above"),
        }

        if self.num_elements == 0 {
            // Matrices are assumed to be square.
            if self.contents == Contents::Matrix {
                self.num_elements = self.num_components;
            } else {
                self.num_elements = 1;
            }
        }

        if self.column_alignment < 1 {
            self.column_alignment = core::cmp::max(self.component_bytes, VERTEX_COLUMN_ALIGNMENT);
        }

        let column_alignment = u16::from(self.column_alignment);
        self.start = self.start.div_ceil(column_alignment) * column_alignment;

        if self.element_stride < 1 {
            self.element_stride = u16::from(self.component_bytes) * u16::from(self.num_components);
        }
        self.total_bytes = u32::from(self.element_stride) * u32::from(self.num_elements);
    }
}

impl GeomVertexColumn {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let name_ref = loader.read_pointer(data)?.unwrap();
        let num_components = data.read_u8()?;
        let numeric_type = NumericType::from(data.read_u8()?);
        let contents = Contents::from(data.read_u8()?);
        let start = data.read_u16()?;
        let column_alignment = match loader.get_minor_version() >= 29 {
            true => data.read_u8()?,
            false => 1,
        };

        let mut column = Self {
            name_ref,
            num_components,
            numeric_type,
            contents,
            start,
            column_alignment,
            ..Default::default()
        };

        column.setup(loader);

        Ok(column)
    }
}
