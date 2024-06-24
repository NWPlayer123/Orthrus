use glam::{dmat4, dvec3, dvec4, DMat4, DVec3, DVec4};

use super::prelude::*;

pub trait DatagramRead {
    fn read(data: &mut Datagram) -> Result<Self, bam::Error>
    where
        Self: Sized;
}

impl DatagramRead for DVec3 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(dvec3(
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
        ))
    }
}

impl DatagramRead for DVec4 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(dvec4(
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
            data.read_float()?,
        ))
    }
}

impl DatagramRead for DMat4 {
    #[inline]
    fn read(data: &mut Datagram) -> Result<Self, bam::Error> {
        Ok(dmat4(
            DVec4::read(data)?,
            DVec4::read(data)?,
            DVec4::read(data)?,
            DVec4::read(data)?,
        ))
    }
}
