use core::{mem::size_of, slice::from_raw_parts_mut};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::sync::mutex::Mutex;
use crate::memory::addr::PAGE_SIZE;
use crate::memory::addr::PhysAddr;
use crate::runtime_err::RuntimeError;

use super::addr::PhysPageNum;
use super::addr::UserAddr;

const USIZE_PER_PAGES: usize = PAGE_SIZE / size_of::<usize>();

#[cfg(not(feature = "board_k210"))]
pub const ADDR_END: usize = 0x81fe0000;

#[cfg(feature = "board_k210")]
pub const ADDR_END: usize = 0x80800000;

// 内存页分配器
pub struct MemoryPageAllocator {
    pub start: usize,
    pub end: usize,
    pub pages: Vec<bool>
}


// 添加内存页分配器方法
impl MemoryPageAllocator {
    // 创建内存分配器结构
    fn new() -> Self {
        MemoryPageAllocator {
            start: 0,
            end: 0,
            pages: vec![]
        }
    }

    // 初始化内存分配器
    fn init(&mut self, start: usize, end: usize) {
        self.start = start;
        self.end = end;
        self.pages = vec![false;(end - start) / PAGE_SIZE];
        info!("初始化页式内存管理, 页表数: {}", self.pages.capacity());
    }

    // 申请内存
    pub fn alloc(&mut self) -> Result<PhysPageNum, RuntimeError> {
        for i in 0..self.pages.len() {
            if !self.pages[i] {
                self.pages[i] = true;
                let page = PhysPageNum::from((self.start >> 12) + i);
                init_pages(page, 1);
                return Ok(page);
            }
        }
        Err(RuntimeError::NoEnoughPage)
    }

    // 取消分配页
    #[allow(unused)]
    pub fn dealloc(&mut self, page: PhysPageNum) {
        let index = usize::from(page) - (self.start >> 12); 
        if let Some(_) = self.pages.get(index) {
            self.pages[index] = false;
        }
    }

    // 申请多个页
    pub fn alloc_more(&mut self, pages: usize) -> Result<PhysPageNum, RuntimeError> {
        let mut i = self.pages.len() - 1;
        let mut value = 0;
        loop {
            if !self.pages[i] {
                value += 1;
            } else {
                value = 0;
            }

            if value >= pages {
                self.pages[i..i+pages].fill(true);
                let page = PhysPageNum::from((self.start >> 12) + i);
                init_pages(page, pages);
                // debug!("申请地址: {:#x}  page_num: {}", page.0, pages);
                return Ok(page);
            }
            if i == 0 { break; }

            // 进行下一个计算
            i-=1;
        }
        Err(RuntimeError::NoEnoughPage)
        // alloc_more_front(pages)
    }

    // 释放多个页
    pub fn dealloc_more(&mut self, page: PhysPageNum, pages: usize) {
        let index = usize::from(page) - (self.start >> 12); 
        if let Some(_) = self.pages.get(index) {
            for i in 0..pages {
                self.pages[index + i] = false;
            }
        }
    }
}

lazy_static! {
    pub static ref PAGE_ALLOCATOR: Mutex<MemoryPageAllocator> = Mutex::new(MemoryPageAllocator::new());
}

pub fn init_pages(page: PhysPageNum, num: usize) {
    unsafe { from_raw_parts_mut(PhysAddr::from(page).0 as *mut usize, USIZE_PER_PAGES * num) }.fill(0);
}

pub fn alloc() -> Result<PhysPageNum, RuntimeError> {
    PAGE_ALLOCATOR.lock().alloc()
}

pub fn alloc_more(pages: usize) -> Result<PhysPageNum, RuntimeError> {
    PAGE_ALLOCATOR.lock().alloc_more(pages)
}

pub fn dealloc_more(page: PhysPageNum, pages: usize) {
    PAGE_ALLOCATOR.lock().dealloc_more(page, pages)
}

pub fn get_free_page_num() -> usize {
    let page_allocator = PAGE_ALLOCATOR.lock();
    let mut last_pages = 0;
    for i in &page_allocator.pages {
        if !i {
            last_pages=last_pages+1;
        }
    }
    last_pages
}
pub fn init() {
    extern "C"{
        fn end();
    }
    // 初始化页表 Vector中每一个元素代表一个页表 通过这种方法来分配页表
    PAGE_ALLOCATOR.lock().init(end as usize, ADDR_END);
}

impl<T> UserAddr<T> {
    pub fn transfer(&self) -> &'static mut T {
        PhysAddr::from(self.0 as usize).tranfer::<T>()
    }

    // 读取一定长度的内存
    pub fn transfer_vec(&self, count: usize) -> &'static mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.0, count) }
    }

    // 读取内存直到发生结束条件
    pub fn transfer_until(&self, until: fn(&T) -> bool) -> &'static mut [T] {
        let addr = self.0 as usize;
        let mut end_addr = addr;
        let count = loop {
            let target_ref = unsafe {(end_addr as *const T).as_ref()}.unwrap();
            if until(target_ref) {
                break (end_addr - addr) / size_of::<T>();
            }
            end_addr += size_of::<T>();
        };
        unsafe { core::slice::from_raw_parts_mut(addr as *mut T, count) }
    }
}

impl UserAddr<u8> {
    // 读取字符串
    pub fn read_string(&self) -> String {
        String::from_utf8_lossy(self.transfer_until(|c| *c == 0)).to_string()
    }
}