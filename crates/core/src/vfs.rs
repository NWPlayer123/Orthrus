//This file still heavily WIP while I decide what to do to "generalize" across many different
// formats.

/*
use core::fmt;

use crate::DataCursor;
use compact_str::CompactString;
use hashbrown::HashMap;


pub enum VirtualNode {
    File(VirtualFile),
    Folder(VirtualFolder),
}

pub struct VirtualFile {
    data: DataCursor,
    metadata: u32,
    timestamp: u32,
}

#[derive(Default)]
pub struct VirtualFolder {
    folders: HashMap<CompactString, VirtualFolder>,
    files: HashMap<CompactString, VirtualFile>,
}

impl VirtualFile {
    pub const fn new(data: DataCursor, metadata: u32, timestamp: u32) -> Self {
        Self {
            data,
            metadata: 0,
            timestamp: 0,
        }
    }
}

impl VirtualFolder {
    pub fn create_file<'a>(
        &mut self,
        mut path: std::iter::Peekable<impl Iterator<Item = &'a str>>,
        file: VirtualFile,
    ) {
        if let Some(segment) = path.next() {
            if path.peek().is_some() {
                let folder_node =
                    self.folders.entry(segment.into()).or_insert_with(|| VirtualFolder {
                        folders: HashMap::new(),
                        files: HashMap::new(),
                    });

                if let folder = folder_node {
                    folder.create_file(path, file);
                }
            } else {
                self.files.insert(segment.into(), file);
            }
        }
    }

    fn debug_display(&self, indent: usize) -> String {
        let mut result = String::new();

        for (name, node) in &self.folders {
            let indentation = "  ".repeat(indent);
            match node {
                VirtualNode::File(file) => {
                    result.push_str(&format!("{indentation}{name}: {file}\n"));
                }
                VirtualNode::Folder(folder) => {
                    result.push_str(&format!("{indentation}{name}: {{\n"));
                    result.push_str(&folder.debug_display(indent + 1));
                    result.push_str(&format!("{indentation}}}\n"));
                }
            }
        }

        result
    }
}

impl fmt::Display for VirtualFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File")
    }
}

impl fmt::Display for VirtualFolder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.debug_display(0))
    }
}
*/
