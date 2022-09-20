use core::cell::RefCell;

use crate::fs::file::FileOP;

pub struct DevRtc(RefCell<bool>);

impl DevRtc {
    pub fn new() -> Self {
        Self(RefCell::new(true))
    }
}

impl FileOP for DevRtc {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        todo!()
    }

    fn read_at(&self, _pos: usize, data: &mut [u8]) -> usize {
        let readable = *self.0.borrow_mut();
        if readable {
            let bytes = b"MemTotal:       8024 kB";
            data[..bytes.len()].copy_from_slice(bytes);
            *self.0.borrow_mut() = false;
            bytes.len()
        } else {
            0
        }
    }

    fn write_at(&self, _pos: usize, _data: &[u8], _count: usize) -> usize {
        todo!("not implemente write dev_rtc")
    }

    fn get_size(&self) -> usize {
       0
    }
}