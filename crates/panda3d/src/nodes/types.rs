use bevy_math::{mat4, quat, uvec3, vec2, vec3, vec4};

use super::prelude::*;

pub trait DatagramRead {
    fn read(data: &mut Datagram) -> Result<Self, bam::Error>
    where
        Self: Sized;
}

impl DatagramRead for Vec2 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(vec2(data.read_float()?, data.read_float()?))
    }
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
        Ok(vec4(data.read_float()?, data.read_float()?, data.read_float()?, data.read_float()?))
    }
}

impl DatagramRead for Mat4 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(mat4(Vec4::read(data)?, Vec4::read(data)?, Vec4::read(data)?, Vec4::read(data)?))
    }
}

impl DatagramRead for Quat {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(quat(data.read_float()?, data.read_float()?, data.read_float()?, data.read_float()?))
    }
}
