use core::ops::{Deref, DerefMut};

use super::prelude::*;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct CollisionPolygon {
    pub inner: CollisionPlane,
    // (Point, Vector) pair
    pub points: Vec<(Vec2, Vec2)>,
    pub to_2d_matrix: Mat4,
}

impl Node for CollisionPolygon {
    #[inline]
    fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let inner = CollisionPlane::create(loader, data)?;

        let point_count = data.read_u16()?;
        let mut points = Vec::with_capacity(point_count as usize);
        for _ in 0..point_count {
            points.push((Vec2::read(data)?, Vec2::read(data)?));
        }

        let to_2d_matrix = Mat4::read(data)?;

        if loader.get_minor_version() < 13 {
            unimplemented!("I don't have any BAM files this old, message me - error in CollisionPolygon");
            //TODO: need to wind vertices the other way
        }

        Ok(Self { inner, points, to_2d_matrix })
    }
}

impl GraphDisplay for CollisionPolygon {
    fn write_data(
        &self, label: &mut impl core::fmt::Write, connections: &mut Vec<u32>, is_root: bool,
    ) -> Result<(), bam::Error> {
        // Header
        if is_root {
            write!(label, "{{CollisionPolygon|")?;
        }

        // Fields
        self.inner.write_data(label, connections, false)?;
        write!(label, "|{{points")?;
        for point in &self.points {
            write!(label, "|{}\\n{}", point.0, point.1)?;
        }
        write!(label, "}}|")?;
        write!(
            label,
            "{{to_2d_matrix|{}\\n{}\\n{}\\n{}}}",
            self.to_2d_matrix.w_axis,
            self.to_2d_matrix.x_axis,
            self.to_2d_matrix.y_axis,
            self.to_2d_matrix.z_axis
        )?;

        // Footer
        if is_root {
            write!(label, "}}")?;
        }
        Ok(())
    }
}

impl Deref for CollisionPolygon {
    type Target = CollisionPlane;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CollisionPolygon {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
