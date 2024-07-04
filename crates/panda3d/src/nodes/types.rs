use super::prelude::*;

use bevy_math::{mat4, uvec3, vec3, vec4};

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

impl DatagramRead for UVec3 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(uvec3(data.read_u32()?, data.read_u32()?, data.read_u32()?))
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
