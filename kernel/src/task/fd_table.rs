use core::borrow::Borrow;

use alloc::rc::Rc;
use hashbrown::HashMap;
use crate::fs::file::FileOP;
use crate::fs::file::File;
use crate::fs::stdio::StdIn;
use crate::fs::stdio::StdOut;
use crate::fs::stdio::StdErr;
use crate::runtime_err::RuntimeError;
use crate::memory::addr::UserAddr;

pub const FD_NULL: usize = 0xffffffffffffff9c;
pub const FD_CWD: usize = -100 as isize as usize;
pub const FD_RANDOM: usize = usize::MAX;

#[derive(Clone)]
pub struct FileDesc {
    pub offset: usize,
    pub file: Rc<dyn FileOP>
}

impl FileDesc {
    pub fn new(file: Rc<dyn FileOP>) -> Self {
        Self {
            offset: 0,
            file
        }
    }

    pub fn readable(&self) -> bool {
        self.file.readable()
    }

    pub fn writeable(&self) -> bool {
        self.file.writeable()
    }

    pub fn get_size(&self) -> usize {
        self.file.get_size()
    }

    pub fn available(&self) -> usize {
        self.get_size() - self.offset
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let read_len = self.file.read_at(*self.offset.borrow(), buf);
        self.offset += read_len;
        read_len
    }

    pub fn write(&self, buf: &[u8], count: usize) -> usize {
        let pos = self.file.get_size();
        let write_len = self.file.write_at(pos, buf, count);
        write_len
    }

    pub fn downcast<T:'static>(&self) -> Result<Rc<T>, Rc<dyn FileOP>> {
        self.file.clone().downcast()
    }

    pub fn lseek(&mut self, offset: usize, whence: usize) -> usize {
        let file_size = self.file.get_size();

        self.offset = match whence {
            // SEEK_SET
            0 => { 
                if self.offset < file_size {
                    self.offset
                } else {
                    file_size
                }
            }
            // SEEK_CUR
            1 => { 
                if self.offset + offset < file_size {
                    self.offset + offset
                } else {
                    file_size
                }
            }
            // SEEK_END
            2 => {
                file_size + self.offset
            }
            _ => { 0 }
        };
        self.offset
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct IoVec {
    pub iov_base: UserAddr<u8>,
    pub iov_len: usize
}

#[derive(Clone)]
pub struct FDTable(HashMap<usize, FileDesc>);

impl FDTable {
    pub fn new() -> Self {
        let mut map:HashMap<usize, FileDesc> = HashMap::new();
        map.insert(0, FileDesc::new(Rc::new(StdIn)));
        map.insert(1, FileDesc::new(Rc::new(StdOut)));
        map.insert(2, FileDesc::new(Rc::new(StdErr)));
        Self(map)
    }

    // 申请fd
    pub fn alloc(&mut self) -> usize {
        (0..).find(|fd| !self.0.contains_key(fd)).unwrap()
    }

    // 申请fd
    pub fn alloc_sock(&mut self) -> usize {
        (50..).find(|fd| !self.0.contains_key(fd)).unwrap()
    }

    // 释放fd
    pub fn dealloc(&mut self, index: usize) {
        self.0.remove(&index);
    }

    // 获取fd内容
    pub fn get(&mut self, index: usize) -> Result<&mut FileDesc, RuntimeError> {
        self.0.get_mut(&index).ok_or(RuntimeError::NoMatchedFileDesc)
    }

    // 获取fd内容
    pub fn get_file(&self, index: usize) -> Result<Rc<File>, RuntimeError> {
        let value = self.0.get(&index).cloned().ok_or(RuntimeError::NoMatchedFileDesc)?;
        value.file.downcast::<File>().map_err(|_| RuntimeError::NoMatchedFile)
    }

    // 设置fd内容
    pub fn set(&mut self, index: usize, value: FileDesc) {
        self.0.insert(index, value);
    }

    // 加入描述符
    pub fn push(&mut self, value: FileDesc) -> usize {
        let index = self.alloc();
        // if index > 41 { return EMFILE; }
        self.set(index, value);
        index
    }

    // 加入描述符
    pub fn push_sock(&mut self, value: FileDesc) -> usize {
        let index = self.alloc_sock();
        self.set(index, value);
        index
    }
}