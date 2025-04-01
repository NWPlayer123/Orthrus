//! Tools for working with X.509 certificates and signed data.

use der::{Decode, Reader, Result, SliceReader};
use x509_cert::certificate::Certificate;

/// Parses X.509 certificate data, returning the valid [`Certificate`] and how many bytes remain
/// after parsing.
///
/// This is intended to be used as an analog for `d2i_X509` from the OpenSSL API, allowing you to
/// parse a blob containing certificate data without knowing its actual length.
///
/// # Errors
/// Returns an error if `bytes` is larger than `0xFFF_FFFF`, or if the decoding fails. See
/// [`der::ErrorKind`] for more details.
// TODO: replace with der::from_der_partial once der 0.8 lands
pub fn read_certificate(bytes: &[u8]) -> Result<(Certificate, usize)> {
    // SliceReader will only fail if larger than 0xFFF_FFFF.
    let mut reader = SliceReader::new(bytes)?;
    // Decoding can be any of a number of different errors, just pass it along.
    let certificate = Certificate::decode(&mut reader)?;
    // This will always be able to fit in a usize, so just unwrap it.
    let remaining: usize = reader.remaining_len().try_into().unwrap();
    Ok((certificate, remaining))
}
