use core::cell::RefCell;

use crate::fs::file::FileOP;

pub struct ProcMounts(RefCell<bool>);

impl ProcMounts {
    pub fn new() -> Self {
        Self(RefCell::new(true))
    }
}

impl FileOP for ProcMounts {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        todo!()
    }

    fn read_at(&self, _pos: usize, data: &mut [u8]) -> usize {
        let readable = *self.0.borrow_mut();
        if readable {
            let bytes = b"fs / fs rw,nosuid,nodev,noexec,relatime 0 0";
            data[..bytes.len()].copy_from_slice(bytes);
            *self.0.borrow_mut() = false;
            bytes.len()
        } else {
            0
        }
    }

    fn write_at(&self, _pos: usize, _data: &[u8], _count: usize) -> usize {
        todo!("not implemente write proc_mounts")
    }

    fn get_size(&self) -> usize {
        0
    }
}