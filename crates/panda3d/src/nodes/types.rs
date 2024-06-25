use glam::{mat4, vec3, vec4, Mat4, Vec3, Vec4};

use super::prelude::*;

pub trait DatagramRead {
    fn read(data: &mut Datagram) -> Result<Self, bam::Error>
    where
        Self: Sized;
}

impl DatagramRead for Vec3 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(vec3(data.read_float()?, data.read_float()?, data.read_float()?))
    }
}

impl DatagramRead for Vec4 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(vec4(
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
        ))
    }
}

impl DatagramRead for Mat4 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(mat4(
            Vec4::read(data)?,
            Vec4::read(data)?,
            Vec4::read(data)?,
            Vec4::read(data)?,
        ))
    }
}
