//use hashbrown::HashMap;
//use std::time::{Duration, SystemTime, UNIX_EPOCH};

/* This whole file will most definitely change in the future as more params
 * are needed */

pub enum VirtualNode {
    File(VirtualFile),
    Directory(VirtualDirectory),
}

impl VirtualNode {
    #[must_use]
    pub fn new_directory(name: String) -> Self {
        VirtualNode::Directory(VirtualDirectory { _name: name })
    }
}

pub struct VirtualFile {}

pub struct VirtualDirectory {
    _name: String,
}

pub struct VirtualFileSystem {
    _root: VirtualNode,
}

impl VirtualFileSystem {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _root: VirtualNode::new_directory("/".to_string()),
        }
    }
}

impl Default for VirtualFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Mountable {
    fn mount(&self, vfs: &mut VirtualFileSystem, mode: MountMode) -> Result<(), String>;
}

pub enum MountMode {
    ReadOnly,
    ReadWrite,
}
