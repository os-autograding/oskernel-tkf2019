use core::any::{Any, TypeId};

use alloc::rc::Rc;
use core::cell::RefCell;


use crate::memory::mem_map::MemMap;
use crate::runtime_err::RuntimeError;
use crate::memory::addr::{get_buf_from_phys_page, get_pages_num, PAGE_SIZE, VirtAddr, PhysPageNum};
use crate::memory::page::alloc_more;
use crate::memory::page_table::{PageMappingManager, PTEFlags};

use super::filetree::INode;

#[allow(unused)]
pub mod fcntl_cmd {
    /// dup
    pub const DUPFD: usize = 0;
    /// get close_on_exec
    pub const GETFD: usize = 1;
    /// set/clear close_on_exec
    pub const SETFD: usize = 2;
    /// get file->f_flags
    pub const GETFL: usize = 3;
    /// set file->f_flags
    pub const SETFL: usize = 4;
    /// Get record locking info.
    pub const GETLK: usize = 5;
    /// Set record locking info (non-blocking).
    pub const SETLK: usize = 6;
    /// Set record locking info (blocking).
    pub const SETLKW: usize = 7;
    /// like F_DUPFD, but additionally set the close-on-exec flag
    pub const DUPFD_CLOEXEC: usize = 0x406;
}

// 文件类型
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FileType {
    File,           // 文件
    VirtFile,       // 虚拟文件
    Directory,      // 文件夹
    Device,         // 设备
    Pipeline,       // 管道
    None            // 空
}

#[repr(C)]
pub struct Kstat {
	pub st_dev: u64,			// 设备号
	pub st_ino: u64,			// inode
	pub st_mode: u32,			// 设备mode
	pub st_nlink: u32,			// 文件links
	pub st_uid: u32,			// 文件uid
	pub st_gid: u32,			// 文件gid
	pub st_rdev: u64,			// 文件rdev
	pub __pad: u64,				// 保留
	pub st_size: u64,			// 文件大小
	pub st_blksize: u32,		// 占用块大小
	pub __pad2: u32,			// 保留
	pub st_blocks: u64,			// 占用块数量
	pub st_atime_sec: u64,		// 最后访问秒
	pub st_atime_nsec: u64,		// 最后访问微秒
	pub st_mtime_sec: u64,		// 最后修改秒
	pub st_mtime_nsec: u64,		// 最后修改微秒
	pub st_ctime_sec: u64,		// 最后创建秒
	pub st_ctime_nsec: u64,		// 最后创建微秒
}

pub trait FileOP: Any {
	fn readable(&self) -> bool;
	fn writeable(&self) -> bool;
	fn read_at(&self, pos: usize, data: &mut [u8]) -> usize;
	fn write_at(&self, pos: usize, data: &[u8], count: usize) -> usize;
	fn get_size(&self) -> usize;
}

pub struct File(pub RefCell<FileInner>);

pub struct FileInner {
    pub file: Rc<INode>,
    pub offset: usize,
    pub file_size: usize,
    pub mem_size: usize,
    pub buf: &'static mut [u8],
    pub mem_map: Option<Rc<MemMap>>,
    pub file_type: FileType
}

impl File {
    pub fn new(inode: Rc<INode>) -> Result<Rc<Self>, RuntimeError>{
        if inode.is_dir() {
            Ok(Rc::new(Self(RefCell::new(FileInner {
                file: inode,
                offset: 0,
                file_size: 0,
                buf: get_buf_from_phys_page(PhysPageNum::from(0x80020usize), 0),
                mem_size: 0,
                mem_map: None,
                file_type: FileType::Directory
            }))))
        } else if inode.is_virt_file() {
            Ok(Rc::new(Self(RefCell::new(FileInner {
                file: inode,
                offset: 0,
                file_size: 0,
                buf: get_buf_from_phys_page(PhysPageNum::from(0x80020usize), 0),
                mem_size: 0,
                mem_map: None,
                file_type: FileType::VirtFile
            }))))
        } else {
            // 申请页表存储程序
            let elf_pages = get_pages_num(inode.get_file_size());
            // 申请页表
            let elf_phy_start = alloc_more(elf_pages)?;
            let mem_map = MemMap::exists_page(elf_phy_start, elf_phy_start.0.into(),
                elf_pages, PTEFlags::VRWX);
            // 获取缓冲区地址并读取
            let buf = get_buf_from_phys_page(elf_phy_start, elf_pages);
            inode.read_to(buf)?;
            let file_size = inode.get_file_size();
            Ok(Rc::new(Self(RefCell::new(FileInner {
                file: inode,
                offset: 0,
                file_size,
                buf,
                mem_size: elf_pages * PAGE_SIZE,
                mem_map: Some(mem_map),
                file_type: FileType::File
            }))))
            
        }
    }

    pub fn cache(inode: Rc<INode>) -> Result<Rc<Self>, RuntimeError>{
        // 申请页表存储程序
        let elf_pages = get_pages_num(inode.get_file_size());
        // 申请页表
        // let elf_phy_start = alloc_more_front(elf_pages)?;
        let elf_phy_start = alloc_more(elf_pages)?;
        let mem_map = MemMap::exists_page(elf_phy_start, elf_phy_start.0.into(),
            elf_pages, PTEFlags::VRWX);
        // 获取缓冲区地址并读取
        let buf = get_buf_from_phys_page(elf_phy_start, elf_pages);
        inode.read_to(buf)?;
        let file_size = inode.get_file_size();
        Ok(Rc::new(Self(RefCell::new(FileInner {
            file: inode,
            offset: 0,
            file_size,
            buf,
            mem_size: elf_pages * PAGE_SIZE,
            mem_map: Some(mem_map),
            file_type: FileType::File
        }))))
        
    }

    pub fn get_inode(&self) -> Rc<INode> {
        let inner = self.0.borrow_mut();
        inner.file.clone()
    }

    pub fn copy_to(&self, offset: usize, buf: &mut [u8]) {
        let inner = self.0.borrow_mut();
        let mut len = inner.buf.len() - offset;
        if len > buf.len() { len = buf.len(); }
        buf[..len].copy_from_slice(&inner.buf[offset..offset + len]);
    }

    pub fn mmap(&self, pmm: Rc<PageMappingManager>, virt_addr: VirtAddr) -> Result<(), RuntimeError>{
        let inner = self.0.borrow_mut();
        let mem_map = inner.mem_map.clone().ok_or(RuntimeError::NoMatchedFile)?;
        pmm.add_mapping_range(mem_map.ppn.into(), virt_addr, mem_map.page_num * PAGE_SIZE, PTEFlags::UVRWX)
    }

    pub fn entry_next(&self) -> Option<(usize, Rc<INode>)> {
        let mut inner = self.0.borrow_mut();
        let offset = inner.offset;
        let child = {
            let children = &mut inner.file.0.borrow_mut().children;
            if offset >= children.len() {
                return None;
            }
            children[offset].clone()
        };
        inner.offset += 1;
        Some((offset, child))
    }

    pub fn get_file_type(&self) -> FileType {
        self.0.borrow_mut().file_type
    }
}

impl FileOP for File {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read_at(&self, pos: usize, data: &mut [u8]) -> usize {
        let inner = self.0.borrow_mut();
        let remain = inner.file_size - pos;
        let len = if remain < data.len() { remain } else { data.len() };
        data[..len].copy_from_slice(&inner.buf[pos..pos + len]);
        len
    }

    fn write_at(&self, pos: usize, data: &[u8], count: usize) -> usize {
        let mut inner = self.0.borrow_mut();
        if inner.file_type == FileType::File {
            let end = pos + count;
            if end >= inner.mem_size {
                panic!("无法写入超出部分");
            }

            let start = pos;
            inner.buf[start..end].copy_from_slice(&data);

            // inner.offset += count;
            // // 需要更新文件数据
            // if inner.offset >= inner.file_size {
            //     inner.file_size = inner.offset;
            //     let _file_size = inner.file_size;
            //     let _inode = inner.file.0.borrow_mut();
            // }
            count
        } else {
            // 写入虚拟文件
            count
        }
    }

    fn get_size(&self) -> usize {
        self.0.borrow_mut().file_size
    }
}

impl dyn FileOP {
    pub fn is<T: 'static>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }
    pub fn downcast<T: 'static>(self: Rc<Self>) -> Result<Rc<T>,Rc<Self>> {
        debug!("type_id: {:?}   self type_id: {:?}   file: {:?}", 
            TypeId::of::<T>(), self.type_id(), TypeId::of::<File>());
        if self.is::<T>() {
            unsafe {
                Ok(Rc::from_raw(Rc::into_raw(self) as _))
            }
        } else {
            Err(self)
        }
    }
}