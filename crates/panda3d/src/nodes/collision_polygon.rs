use super::prelude::*;

#[derive(Debug, Default)]
#[expect(dead_code)]
pub(crate) struct CollisionPolygon {
    plane: CollisionPlane,
    points: Vec<(Vec2, Vec2)>,
    to_2d_matrix: Mat4,
}

impl CollisionPolygon {
    #[inline]
    pub fn create(loader: &mut BinaryAsset, data: &mut Datagram) -> Result<Self, bam::Error> {
        let plane = CollisionPlane::create(loader, data)?;

        let point_count = data.read_u16()?;
        let mut points = Vec::with_capacity(point_count as usize);
        for _ in 0..point_count {
            points.push((Vec2::read(data)?, Vec2::read(data)?));
        }

        let to_2d_matrix = Mat4::read(data)?;
        if loader.get_minor_version() < 13 {
            panic!("I don't have any BAM files this old, message me");
            //TODO: need to wind vertices the other way
        }
        Ok(Self { plane, points, to_2d_matrix })
    }
}
