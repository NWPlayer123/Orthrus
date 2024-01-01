/// This module is for an identification system to allow for types to return inforamtion if they
/// recognize a given file.

#[non_exhaustive]
pub struct FileInfo {
    pub info: String,
    pub payload: Option<Box<[u8]>>,
}

impl FileInfo {
    pub fn new(info: String, payload: Option<Box<[u8]>>) -> Self {
        FileInfo { info, payload }
    }
}

impl Default for FileInfo {
    fn default() -> Self {
        FileInfo {
            info: String::new(),
            payload: None,
        }
    }
}

pub trait FileIdentifier {
    fn identify(data: &[u8]) -> Option<FileInfo>;
    fn identify_deep(data: &[u8]) -> Option<FileInfo> {
        Self::identify(data)
    }
}

pub type IdentifyFn = fn(&[u8]) -> Option<FileInfo>;
