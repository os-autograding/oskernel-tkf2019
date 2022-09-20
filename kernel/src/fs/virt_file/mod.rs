use alloc::string::String;

use crate::{memory::mem_set::MemSet, interrupt::timer::TimeSpec};

use super::file::FileOP;

#[derive(Clone)]
pub struct VirtFile {
    pub filename: String,
    pub mem_set: MemSet,
    pub file_size: usize,
    pub mtime: TimeSpec,
    pub atime: TimeSpec,
    pub ctime: TimeSpec
}

impl VirtFile {
    pub fn new(filename: String) -> Self {
        let now = TimeSpec::now();
        Self {  
            filename,
            mem_set: MemSet::new(),
            mtime: now,
            atime: now,
            ctime: now,
            file_size: 0
        }
    }
}

impl FileOP for VirtFile {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read_at(&self, pos: usize, data: &mut [u8]) -> usize {
        todo!()
    }

    fn write_at(&self, pos: usize, data: &[u8], count: usize) -> usize {
        todo!("not implemente write virt_file")
    }

    fn get_size(&self) -> usize {
        self.file_size
    }
}