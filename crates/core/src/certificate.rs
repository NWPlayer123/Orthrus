use x509_cert::certificate::{CertificateInner, Rfc5280};
use der::{Decode, Reader, Result, SliceReader, Length};

#[derive(Debug)]
pub struct Certificate {
    pub certificate: CertificateInner<Rfc5280>,
    pub remaining_len: Length,
}

impl<'a> Decode<'a> for Certificate {
    fn decode<R: Reader<'a>>(reader: &mut R) -> Result<Self> {
        let inner = CertificateInner::<Rfc5280>::decode(reader)?;
        Ok(Certificate { certificate: inner, remaining_len: Length::new(0) })
    }

    fn from_der(bytes: &'a [u8]) -> Result<Self> {
        let mut reader = SliceReader::new(bytes)?;
        let mut result = Self::decode(&mut reader)?;
        result.remaining_len = reader.remaining_len();
        Ok(result)
    }
}
