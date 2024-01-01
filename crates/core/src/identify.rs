/// This module is for an identification system to allow for types to return inforamtion if they recognize a given file.
///

#[non_exhaustive]
struct FileInfo {
    info: String,
    payload: Option<Box<[u8]>>,
}

trait FileIdentifier {
    fn identify(data: &[u8]) -> Option<FileInfo>;
    fn identify_deep(data: &[u8]) -> Option<FileInfo> {
        Self::identify(data)
    }
}
