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
