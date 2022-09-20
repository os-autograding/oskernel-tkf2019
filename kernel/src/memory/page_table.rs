use core::arch::asm;
use core::slice::from_raw_parts_mut;
use core::slice;
use _core::cell::RefCell;
use bitflags::*;

use crate::memory::addr::PhysAddr;
use crate::sync::mutex::Mutex;
use crate::runtime_err::RuntimeError;

use super::addr::PhysPageNum;
use super::addr::VirtAddr;
use super::addr::PAGE_PTE_NUM;
use super::addr::PAGE_SIZE;
use super::addr::VirtPageNum;
use super::page::ADDR_END;
use super::page::PAGE_ALLOCATOR;
use super::mem_map::MemMap;
use super::mem_set::MemSet;

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;       // 是否合法 为1合法
        const R = 1 << 1;       // 可读
        const W = 1 << 2;       // 可写
        const X = 1 << 3;       // 可执行
        const U = 1 << 4;       // 处于U特权级下是否允许被访问
        const G = 1 << 5;       // 
        const A = 1 << 6;       // 是否被访问过
        const D = 1 << 7;       // 是否被修改过
        const NONE = 0;
        const VRWX = 0xf;
        const UVRWX = 0x1f;
    }
}

const ENTRY_NUM_PER_PAGE: usize = 512;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    #[allow(unused)]
    pub fn empty() -> Self {
        PageTableEntry {
            bits: 0,
        }
    }

    // 获取ppn
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    // 获取标志
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    // 判断是否为页表
    #[allow(unused)]
    pub fn is_valid_pte(&self) -> bool {
        self.flags().contains(PTEFlags::V) && self.flags() & PTEFlags::VRWX != PTEFlags::V
    }

    // 判断是否为页目录
    pub fn is_valid_pd(&self) -> bool {
        self.flags().contains(PTEFlags::V) && self.flags() & PTEFlags::VRWX == PTEFlags::V
    }

    // 获取可更换ptr
    pub unsafe fn get_mut_ptr_from_phys(addr:PhysAddr) -> *mut Self {
        usize::from(addr) as *mut Self
    }

    pub fn get_vec_from_phys<'a>(addr: PhysAddr) -> &'a mut [PageTableEntry] {
        unsafe {
            slice::from_raw_parts_mut(usize::from(addr) as *mut Self, ENTRY_NUM_PER_PAGE)
        }
    }
}

#[derive(Clone)]
pub enum PagingMode {
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9
}

#[derive(Clone)]
pub struct PageMappingManager {
    pub paging_mode: PagingMode,
    pub mem_set: RefCell<MemSet>,
    pub pte: PageMapping
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageMapping(pub usize);

impl From<usize> for PageMapping {
    fn from(addr: usize) -> Self {
        PageMapping(addr)
    }
}

impl From<PhysAddr> for PageMapping {
    fn from(addr: PhysAddr) -> Self {
        PageMapping(addr.0)
    }
}

impl From<PageMapping> for PhysPageNum {
    fn from(addr: PageMapping) -> Self {
        PhysPageNum::from(PhysAddr::from(addr.0))
    }
}

impl From<PageMapping> for usize {
    fn from(addr: PageMapping) -> Self {
        addr.0
    }
}

// PageMapping 
impl PageMapping {
    pub fn new(addr: PhysAddr) -> PageMapping {
        PageMapping(addr.0)
    }

    // 初始化页表
    pub fn alloc_pte(level: usize) -> Result<PhysPageNum, RuntimeError> {
        let page = PAGE_ALLOCATOR.lock().alloc()?;
        let pte = unsafe {
            from_raw_parts_mut(usize::from(page.to_addr()) as *mut PageTableEntry, PAGE_PTE_NUM)
        };
        for i in 0..PAGE_PTE_NUM {
            pte[i] = PageTableEntry::new(PhysPageNum::from(i << (level*9)), PTEFlags::VRWX);
        }
        Ok(page)
    }

    // 删除mapping
    pub fn remove_mapping(&self, virt_addr: VirtAddr) {
        // 如果没有pte则申请pte
        if usize::from(self.0) == 0 {
            return;
        }

        // 得到 列表中的项
        let l2_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(self.0)).add(virt_addr.l2())
        };
        let mut l2_pte = unsafe { l2_pte_ptr.read() };

        // 判断 是否是页表项 如果是则申请一个页防止其内容
        if !l2_pte.is_valid_pd() {
            // 创建一个页表放置二级页目录 并写入一级页目录的项中
            l2_pte = PageTableEntry::new(PhysPageNum::from(PhysAddr::from(Self::alloc_pte(1).unwrap())), PTEFlags::V);
            // 写入列表
            unsafe {l2_pte_ptr.write(l2_pte)};
        }

        let l1_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(l2_pte.ppn())).add(virt_addr.l1())
        };
        let mut l1_pte = unsafe {l1_pte_ptr.read()};

        // 判断 是否有指向下一级的页表
        if !l1_pte.is_valid_pd(){
            l1_pte = PageTableEntry::new(PhysPageNum::from(PhysAddr::from(Self::alloc_pte(0).unwrap())), PTEFlags::V);
            unsafe{l1_pte_ptr.write(l1_pte)};
        }
        
        // 写入映射项
        unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(l1_pte.ppn()))
                .add(virt_addr.l0()).write(PageTableEntry::new(PhysPageNum::from(PhysPageNum::from(virt_addr.l0() << 18)), PTEFlags::VRWX));
        }
    }

    // 获取物理地址
    pub fn get_phys_addr(&self, virt_addr: VirtAddr) -> Result<PhysAddr, RuntimeError> {
        // 如果没有pte则申请pte
        if usize::from(self.0) == 0 {
            return Err(RuntimeError::NoMatchedAddr);
        }

        // 得到 列表中的项
        let l2_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(self.0)).add(virt_addr.l2())
        };
        let l2_pte = unsafe { l2_pte_ptr.read() };

        // 判断 是否有指向下一级的页表
        if !l2_pte.flags().contains(PTEFlags::V) {
            return Err(RuntimeError::NoMatchedAddr);
        }
        if l2_pte.flags() & PTEFlags::VRWX != PTEFlags::V {
            return Ok(PhysAddr::from(virt_addr.page_offset() | (virt_addr.l0() << 12) | (virt_addr
                .l1() << 21) | (usize::from(l2_pte.ppn()) << 12)));
        }

        let l1_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(l2_pte.ppn())).add(virt_addr.l1())
        };
        let l1_pte = unsafe { l1_pte_ptr.read() };

        // 判断 是否有指向下一级的页表
        if !l1_pte.flags().contains(PTEFlags::V) {
            return Err(RuntimeError::NoMatchedAddr);
        }
        if l1_pte.flags() & PTEFlags::VRWX != PTEFlags::V {
            return Ok(PhysAddr::from(virt_addr.page_offset() | (virt_addr.l0() << 12) | (usize::from(l1_pte.ppn()) << 12)));
        }

        // 获取pte项
        let l0_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(l1_pte.ppn())).add(virt_addr.l0())
        };
        let l0_pte = unsafe { l0_pte_ptr.read() };
        if !l0_pte.flags().contains(PTEFlags::V) {
            return Err(RuntimeError::NoMatchedAddr);
        }
        Ok(PhysAddr::from(usize::from(PhysAddr::from(l0_pte.ppn())) + virt_addr.page_offset()))
    }

    pub fn get_entry(&self, virt_addr: VirtAddr) -> Result<PageTableEntry, RuntimeError> {
        // 如果没有pte则申请pte
        if usize::from(self.0) == 0 {
            return Err(RuntimeError::NoMatchedAddr);
        }

        // 得到 列表中的项
        let l2_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(self.0)).add(virt_addr.l2())
        };
        let l2_pte = unsafe { l2_pte_ptr.read() };

        // 判断 是否有指向下一级的页表
        if !l2_pte.flags().contains(PTEFlags::V) {
            return Err(RuntimeError::NoMatchedAddr);
        }

        let l1_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(l2_pte.ppn())).add(virt_addr.l1())
        };
        let l1_pte = unsafe { l1_pte_ptr.read() };

        // 判断 是否有指向下一级的页表
        if !l1_pte.flags().contains(PTEFlags::V) {
            return Err(RuntimeError::NoMatchedAddr);
        }

        // 获取pte项
        let l0_pte_ptr = unsafe {
            PageTableEntry::get_mut_ptr_from_phys(PhysAddr::from(l1_pte.ppn())).add(virt_addr.l0())
        };
        let l0_pte = unsafe { l0_pte_ptr.read() };
        if !l0_pte.flags().contains(PTEFlags::V) {
            return Err(RuntimeError::NoMatchedAddr);
        }
        Ok(l0_pte)
    }

    pub fn add_mapping(&self, ppn: PhysPageNum, vpn: VirtPageNum, flags: PTEFlags) -> Result<MemSet, RuntimeError>{
        let mut mem_set = MemSet::new();
        let l_vec = vpn.get_l_vec();

        let pte_vec = PageTableEntry::get_vec_from_phys(self.0.into());
        let mut l2_pte = pte_vec[l_vec[0]];

        // 判断 是否是页表项 如果是则申请一个页防止其内容
        if !l2_pte.is_valid_pd() {
            // 创建一个页表放置二级页目录 并写入一级页目录的项中
            let pte_ppn = Self::alloc_pte(1)?;
            mem_set.inner().push(MemMap::pte_container(pte_ppn));
            l2_pte = PageTableEntry::new(pte_ppn, PTEFlags::V);
            // 写入列表
            pte_vec[l_vec[0]] = l2_pte;
        }
        let pte_vec = PageTableEntry::get_vec_from_phys(l2_pte.ppn().into());
        let mut l1_pte = pte_vec[l_vec[1]];

        // 判断 是否有指向下一级的页表
        if !l1_pte.is_valid_pd(){
            let pte_ppn = Self::alloc_pte(0)?;
            mem_set.inner().push(MemMap::pte_container(pte_ppn));
            l1_pte = PageTableEntry::new(pte_ppn, PTEFlags::V);
            pte_vec[l_vec[1]] = l1_pte;
        }
        
        let pte_vec = PageTableEntry::get_vec_from_phys(l1_pte.ppn().into());
        pte_vec[l_vec[2]] = PageTableEntry::new(ppn, flags);
        Ok(mem_set)
    }

    pub fn add_mapping_by_map(&self, map: &MemMap) -> Result<MemSet, RuntimeError> {
        let mut mem_set = MemSet::new();
        for i in 0..map.page_num {
            mem_set.append(&mut self.add_mapping(map.ppn + i.into(), map.vpn + i.into(), map.flags)?);
        }
        Ok(mem_set)
    }
}


impl PageMappingManager {
    pub fn new() -> Result<Self, RuntimeError> {
        let mut mem_set = MemSet::new();
        let ppn = PageMapping::alloc_pte(2)?;

        mem_set.inner().push(MemMap::pte_page(ppn));

        Ok(PageMappingManager { 
            paging_mode: PagingMode::Sv39, 
            pte: PhysAddr::from(ppn).into(),
            mem_set: RefCell::new(mem_set)
        })
    }

    // 获取pte
    pub fn get_pte(&self) -> usize {
        self.pte.into()
    }

    // 添加mapping
    pub fn add_mapping(&self, ppn: PhysPageNum, vpn: VirtPageNum, flags: PTEFlags) -> Result<(), RuntimeError>{
        let mut target = self.pte.add_mapping(ppn, vpn, flags)?;
        self.add_mem_set(&mut target);
        Ok(())
    }

    pub fn add_mapping_by_map(&self, map: &MemMap) -> Result<(), RuntimeError> {
        let mut target = self.pte.add_mapping_by_map(map)?;
        self.add_mem_set(&mut target);
        Ok(())
    }

    pub fn add_mapping_by_set(&self, map: &MemSet) -> Result<(), RuntimeError> {
        for i in &map.0 {
            let mut target = self.pte.add_mapping_by_map(i)?;
            self.add_mem_set(&mut target);
        }
        Ok(())
    }

    // 添加一个范围内的mapping
    pub fn add_mapping_range(&self, phy_addr: PhysAddr, virt_addr: VirtAddr, size: usize, flags:PTEFlags) -> Result<(), RuntimeError> {        
        let end_addr: usize = virt_addr.0 + size;
        let mut i: usize = virt_addr.0 / PAGE_SIZE * PAGE_SIZE;   // floor get start_page
        loop {
            if i > end_addr { break; }
            let v_offset: usize = i - virt_addr.0;
            self.add_mapping(PhysAddr::from(phy_addr.0 + v_offset).into(), VirtAddr::from(i).into(), flags)?;
            i += PAGE_SIZE;
        }
        Ok(())
    }

    pub fn remove_mapping(&self, virt_addr: VirtAddr) {
        self.pte.remove_mapping(virt_addr)
    }

    // 获取物理地址
    pub fn get_phys_addr(&self, virt_addr: VirtAddr) -> Result<PhysAddr, RuntimeError> {
        self.pte.get_phys_addr(virt_addr)
    }

    // 获取物理地址
    pub fn get_entry(&self, virt_addr: VirtAddr) -> Result<PageTableEntry, RuntimeError> {
        self.pte.get_entry(virt_addr)
    }
    

    // 更改pte
    pub fn change_satp(&self) {
        let satp_addr = (self.paging_mode.clone() as usize) << 60 | usize::from(PhysPageNum::from(self.pte));
        unsafe {
            asm!("csrw satp, {}",
            "sfence.vma", in(reg) satp_addr)
        }
    }

    // 添加内存set
    pub fn add_mem_set(&self, target_mem_set: &mut MemSet) {
        let mut mem_set = self.mem_set.borrow_mut();
        mem_set.append(target_mem_set);
    }

    // 释放内存资源
    pub fn release(&self) {
        let mut mem_set = self.mem_set.borrow_mut();
        mem_set.release();
    }
}

lazy_static! {
    pub static ref KERNEL_PAGE_MAPPING: Mutex<PageMappingManager> = Mutex::new(PageMappingManager::new().unwrap());
}

// 初始化页面映射
pub fn init() {
    {
        let kernel_page = KERNEL_PAGE_MAPPING.force_get();
        
        let mem_map = MemMap::exists_page(0x80000usize.into(), 0x80000usize.into(), 
                (ADDR_END - 0x8000_0000) / PAGE_SIZE, PTEFlags::VRWX | PTEFlags::G | PTEFlags::D | PTEFlags::A);
        kernel_page.add_mapping_by_map(&mem_map).expect("地址申请失败");
    }
    switch_to_kernel_page();
}

pub fn switch_to_kernel_page() {
    let mapping_manager = KERNEL_PAGE_MAPPING.force_get();
    mapping_manager.change_satp();
}