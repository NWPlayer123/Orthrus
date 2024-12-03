//! Identification system that allows types to return information if they recognize a given type.
//!
//! This has two systems: basic identification, which should only perform operations if they won't
//! take a significant amount of time, and "deep identification", which is allowed to perform any
//! computation even if it may take multiple seconds, along with allowing recursion into nested
//! types.

#[cfg(not(feature = "std"))]
use crate::no_std::*;

/// Contains the relevant file info to return after identification.
#[derive(Default)]
#[non_exhaustive]
pub struct FileInfo {
    /// Contains plaintext info about the type, if recognized.
    pub info: String,
    /// Used for returning any inner data if using deep identification.
    pub payload: Option<Box<[u8]>>,
}

impl FileInfo {
    /// Creates a new instance to return information about a file.
    #[must_use]
    #[inline]
    pub const fn new(info: String, payload: Option<Box<[u8]>>) -> Self {
        Self { info, payload }
    }
}

/// Trait that allows for identifying if a byte slice is of the same format as the type.
pub trait FileIdentifier {
    /// Attempts to identify a specific type, and return human-readable info about it.
    #[must_use]
    fn identify(data: &[u8]) -> Option<FileInfo>;

    /// Attempts to identify a specific type and any sub-type, and return human-readable info about
    /// it.
    #[must_use]
    #[inline]
    fn identify_deep(data: &[u8]) -> Option<FileInfo> {
        Self::identify(data)
    }
}

/// Type alias for [`identify`](FileIdentifier::identify) and
/// [`identify_deep`](FileIdentifier::identify_deep).
pub type IdentifyFn = fn(&[u8]) -> Option<FileInfo>;
