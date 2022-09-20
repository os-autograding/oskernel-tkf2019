use core::cell::RefCell;

use crate::fs::file::FileOP;

pub struct EtcAdjtime(RefCell<bool>);

impl EtcAdjtime {
    pub fn new() -> Self {
        Self(RefCell::new(true))
    }
}

impl FileOP for EtcAdjtime {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        todo!()
    }

    fn read_at(&self, _pos: usize, data: &mut [u8]) -> usize {
        let readable = *self.0.borrow_mut();
        if readable {
            let bytes = b"0.000000 1643115317 0.000000\n1643115317\nUTC";
            data[..bytes.len()].copy_from_slice(bytes);
            *self.0.borrow_mut() = false;
            bytes.len()
        } else {
            0
        }
    }

    fn write_at(&self, _pos: usize, _data: &[u8], _count: usize) -> usize {
        todo!("not implemente write adjtime")
    }

    fn get_size(&self) -> usize {
        0
    }
}