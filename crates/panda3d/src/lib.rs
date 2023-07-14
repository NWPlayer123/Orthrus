pub mod multifile;
pub use multifile::Multifile;
/*use orthrus_helper::vfs::VirtualFileSystem;
use std::io;
use std::path::Path;

fn load_multifile(path: &Path) -> io::Result<VirtualFileSystem> {
    let vfs = VirtualFileSystem::new();
    let mut file1 = Multifile::new();
    file1.open_read(path, 0)?;
    Ok(vfs)
}*/
