use crate::console::puts;
use super::file::FileOP;

pub struct StdIn;
pub struct StdOut;
pub struct StdErr;
pub struct StdZero;
pub struct StdNull;

impl FileOP for StdIn {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        false
    }

    fn read_at(&self, _pos: usize, _data: &mut [u8]) -> usize {
        todo!()
    }

    fn write_at(&self, _pos: usize, _data: &[u8], _count: usize) -> usize {
        todo!("not implemente write stdin")
    }

    fn get_size(&self) -> usize {
        0
    }
}

impl FileOP for StdOut {
    fn readable(&self) -> bool {
        false
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read_at(&self, _pos: usize, _data: &mut [u8]) -> usize {
        0
    }

    fn write_at(&self, _pos: usize, data: &[u8], _count: usize) -> usize {
        puts(data);
        data.len()
    }

    fn get_size(&self) -> usize {
        0
    }
}

impl FileOP for StdErr {
    fn readable(&self) -> bool {
        todo!()
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read_at(&self, _pos: usize, _data: &mut [u8]) -> usize {
        todo!()
    }

    fn write_at(&self, _pos: usize, data: &[u8], count: usize) -> usize {
        if count > 0 && data[0] != b's' {
            puts(data);
        }
        count
    }

    fn get_size(&self) -> usize {
        0
    }
}

impl FileOP for StdZero {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        todo!()
    }

    fn read_at(&self, _pos: usize, data: &mut [u8]) -> usize {
        data.fill(0);
        data.len()
    }

    fn write_at(&self, _pos: usize, _data: &[u8], _count: usize) -> usize {
        todo!("not implemente write stzero")
    }

    fn get_size(&self) -> usize {
        0
    }
}

impl FileOP for StdNull {
    fn readable(&self) -> bool {
        false
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read_at(&self, _pos: usize, _data: &mut [u8]) -> usize {
        0
    }

    fn write_at(&self, _pos: usize, _data: &[u8], count: usize) -> usize {
        count
    }

    fn get_size(&self) -> usize {
        0
    }
}
