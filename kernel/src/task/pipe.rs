use core::cell::RefCell;
use alloc::sync::Arc;
use alloc::rc::Rc;
use alloc::vec::Vec;
use crate::fs::file::FileOP;
use super::fd_table::FileDesc;

pub struct PipeBufInner {
    pub buf: Vec<u8>,
    pub read_offset: usize,
    pub write_offset: usize
}

#[derive(Clone)]
pub struct PipeBuf(pub Arc<RefCell<PipeBufInner>>);

impl PipeBuf {
    // 创建pipeBuf
    pub fn new() -> Self {
        Self(Arc::new(RefCell::new(PipeBufInner {
            buf: Vec::new(),
            read_offset: 0,
            write_offset: 0
        })))
    }
    // 读取字节
    pub fn read_at(&self, mut pos: usize, buf: &mut [u8]) -> usize {
        let mut read_index = 0;
        let pipe = self.0.borrow_mut();
        loop {
            if read_index >= buf.len() {
                break;
            }

            if pos < pipe.buf.len() {
                buf[read_index] = pipe.buf[pos];
            } else {
                break;
            }

            pos += 1;
            read_index += 1;
        }
        read_index
    }

    pub fn write_at(&self, mut pos: usize, buf: &[u8], count: usize) -> usize{
        let mut write_index = 0;
        let mut pipe = self.0.borrow_mut();

        if pipe.buf.len() > 4096 {
            let start = pipe.buf.len() - 4096;
            pipe.buf = pipe.buf[start..].to_vec();
        }

        loop {
            if write_index >= buf.len() || write_index >= count {
                break;
            }
            
            // queue.push_back(buf[write_index]);
            pipe.buf.push(buf[write_index]);
            pos += 1;
            write_index += 1;
        }
        // 如果栈过大则重新分配
        write_index
    }

    // 获取可获取的大小
    pub fn available(&self) -> usize {
        let pipe = self.0.borrow_mut();
        pipe.write_offset - pipe.read_offset
    }

    pub fn get_size(&self) -> usize {
        let pipe = self.0.borrow_mut();
        pipe.buf.len()
    }
}

pub struct PipeReader(PipeBuf);

pub struct PipeWriter(PipeBuf);

impl FileOP for PipeReader {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        false
    }

    fn read_at(&self, pos: usize, data: &mut [u8]) -> usize {
        debug!("write?");
        self.0.read_at(pos, data)
    }

    fn write_at(&self, _pos: usize, _data: &[u8], _count: usize) -> usize {
        debug!("write?");
        todo!()
    }

    fn get_size(&self) -> usize {
        // self.0.available()
        self.0.get_size()
    }
}

impl FileOP for PipeWriter {
    fn readable(&self) -> bool {
        false
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read_at(&self, _pos: usize, _data: &mut [u8]) -> usize {
        debug!("write?");
        todo!()
    }

    fn write_at(&self, pos: usize, data: &[u8], count: usize) -> usize {
        debug!("write?");
        self.0.write_at(pos, data, count)
    }

    fn get_size(&self) -> usize {
        self.0.get_size()
    }
}

pub fn new_pipe() -> (FileDesc, FileDesc) {
    let pipe_buf = PipeBuf::new();
    let pipe_reader  = FileDesc::new(Rc::new(PipeReader(pipe_buf.clone())));
    let pipe_writer = FileDesc::new(Rc::new(PipeWriter(pipe_buf.clone())));
    (pipe_reader, pipe_writer)
}