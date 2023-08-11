use core::fmt;

use compact_str::CompactString;
use hashbrown::HashMap;

pub enum VirtualNode<T> {
    File(VirtualFile<T>),
    Folder(VirtualFolder<T>),
}

pub struct VirtualFile<T> {
    data: T,
}

#[derive(Default)]
pub struct VirtualFolder<T> {
    nodes: HashMap<CompactString, VirtualNode<T>>,
}

impl<T> VirtualFile<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> VirtualFolder<T> {
    pub fn create_file<'a>(&mut self, mut path: std::iter::Peekable<impl Iterator<Item = &'a str>>, data: T) {
        if let Some(segment) = path.next() {
            if path.peek().is_some() {
                let folder_node = self.nodes.entry(segment.into())
                    .or_insert_with(|| VirtualNode::Folder(VirtualFolder {
                        nodes: HashMap::new(),
                    }));

                if let VirtualNode::Folder(ref mut folder) = folder_node {
                    folder.create_file(path, data);
                }
            } else {
                self.nodes.insert(segment.into(), VirtualNode::File(VirtualFile { data }));
            }
        }
    }

    pub fn debug_display(&self, indent: usize) {
        for (name, node) in self.nodes.iter() {
            let indentation = "  ".repeat(indent);
            match node {
                VirtualNode::File(file) => log::debug!("{}{}: {}", indentation, name, file),
                VirtualNode::Folder(folder) => {
                    log::debug!("{}{}: {{", indentation, name);
                    folder.debug_display(indent + 1);
                    log::debug!("{}}}", indentation);
                }
            }
        }
    }
}

impl<T> fmt::Display for VirtualFile<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File")
    }
}

impl<T> fmt::Display for VirtualFolder<T> {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug_display(0);
        Ok(())
    }
}
