use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use crate::memory::page_table::PTEFlags;
use crate::memory::page_table::PageMappingManager;
use crate::memory::addr::VirtAddr;
use crate::memory::addr::PAGE_SIZE;
use crate::memory::mem_set::MemSet;
use crate::memory::mem_map::MemMap;
use crate::runtime_err::RuntimeError;


pub const PTR_SIZE: usize = 8;
pub const DEFAULT_STACK_PAGE_NUM: usize = 40;
pub const DEFAULT_STACK_ADDR: usize = 0xf0010000;

#[derive(Clone)]
pub struct UserStack {
    pub bottom: usize,
    pub top: usize,
    pub pointer: usize,
    pub pmm: Rc<PageMappingManager>,
    pub mem_set: MemSet
}

impl UserStack {
    // 创建新的栈
    pub fn new(pmm: Rc<PageMappingManager>) -> Result<Self, RuntimeError> {
        let mut mem_set = MemSet::new();
        let mem_map = MemMap::new((DEFAULT_STACK_ADDR / PAGE_SIZE - DEFAULT_STACK_PAGE_NUM).into(), DEFAULT_STACK_PAGE_NUM, PTEFlags::UVRWX)?;
        pmm.add_mapping_by_map(&mem_map)?;
        mem_set.inner().push(mem_map);
        Ok(UserStack { 
            bottom: DEFAULT_STACK_ADDR, 
            top: DEFAULT_STACK_ADDR - DEFAULT_STACK_PAGE_NUM * PAGE_SIZE,
            pointer: DEFAULT_STACK_ADDR,
            pmm,
            mem_set
        })
    }

    pub fn get_stack_top(&self) -> usize {
        self.pointer
    }

    // 在栈中加入数字
    pub fn push(&mut self, num: usize) -> usize {
        self.pointer -= PTR_SIZE;
        let phys_ptr = self.pmm.get_phys_addr(self.pointer.into()).unwrap().0;
        unsafe {
            (phys_ptr as *mut usize).write(num)
        };
        self.pointer
    }

    // 在栈中加入字符串 并且内存对齐
    pub fn push_arr(&mut self, str: &[u8]) -> usize {
        // 设置 总长度
        let str_len = (str.len() + 1 + (PTR_SIZE - 1)) / PTR_SIZE;
        self.pointer -= PTR_SIZE * str_len;

        let mut phys_ptr = self.pmm.get_phys_addr(self.pointer.into()).unwrap().0;
        let mut virt_ptr = self.pointer;
        for i in 0..str.len() {
            // 写入字节
            unsafe {(phys_ptr as *mut u8).write(str[i])};
            virt_ptr += 1;
            // 如果虚拟地址越界 则重新映射
            if virt_ptr % 4096 == 0 {
                phys_ptr = self.pmm.get_phys_addr(VirtAddr::from(self.pointer)).unwrap().0;
            } else {
                phys_ptr += 1;
            }
        }
        // 写入 \0 作为结束符
        unsafe {(phys_ptr as *mut u8).write(0)};
        self.pointer
    }

    pub fn push_str(&mut self, str: &str) -> usize {
        self.push_arr(str.as_bytes())
    }

    // 在栈中加入指针 内部调用push 后期可额外处理
    pub fn push_ptr(&mut self, ptr: usize) -> usize {
        self.push(ptr)
    }

    pub fn init_args(&mut self, args: Vec<&str>, _envp: Vec<&str>, auxv: BTreeMap<usize, usize>) {
        let envp = self.push_str("LD_LIBRARY_PATH=/");
        let args: Vec<usize> = args.iter().map(|x| self.push_str(x)).collect();
        // auxv top
        self.push(0);

        for (key, value) in auxv {
            self.push(value);
            self.push(key);
        }
        // envp top
        self.push(0);
        self.push(envp);
        // argv top
        self.push(0);    // argv 没有top? 

        // args
        let args_len = args.len();
        for i in args.iter().rev() {
            self.push(i.clone());
        }
        self.push(args_len);
    }

    // 复制数据
    pub fn clone_with_data(&self, pmm: Rc<PageMappingManager>) -> Result<Self, RuntimeError> {
        let mem_set = self.mem_set.clone_with_data()?;

        pmm.add_mapping_by_set(&mem_set)?;

        Ok(UserStack { 
            bottom: self.bottom, 
            top: self.top,
            pointer: self.pointer,
            pmm,
            mem_set
        })
    }

    // 释放资源
    pub fn release(&mut self) {
        self.mem_set.release();
    }

    pub fn alloc_until(&mut self, until_addr: usize) -> Result<(), RuntimeError> {
        loop {
            if until_addr >= self.top { break; }
            let start_page = self.top / PAGE_SIZE - 1;
            let mem_map = MemMap::new(start_page.into(), 1, PTEFlags::UVRWX)?;
            self.pmm.add_mapping_by_map(&mem_map)?;
            self.mem_set.inner().push(mem_map);
            self.top -= PAGE_SIZE;
        }
        Ok(())
    }
}