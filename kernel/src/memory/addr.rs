use core::fmt;
use core::fmt::Debug;
use core::fmt::Formatter;
use core::ops::Add;
use core::slice;
use core::mem::size_of;
use alloc::vec::Vec;

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_PTE_NUM: usize = 512;

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

// 实现从usize转换
impl From<usize> for PhysAddr  {
    fn from(addr: usize) -> Self {
        PhysAddr(addr)
    }
}

impl From<usize> for PhysPageNum  {
    fn from(addr: usize) -> Self {
        PhysPageNum(addr)
    }
}

impl From<usize> for VirtPageNum  {
    fn from(addr: usize) -> Self {
        VirtPageNum(addr)
    }
}

impl From<usize> for VirtAddr  {
    fn from(addr: usize) -> Self {
        VirtAddr(addr)
    }
}

// 实现从u64转换
impl From<u64> for PhysAddr  {
    fn from(addr: u64) -> Self {
        PhysAddr(addr as usize)
    }
}

impl From<u64> for PhysPageNum  {
    fn from(addr: u64) -> Self {
        PhysPageNum(addr as usize)
    }
}

impl From<u64> for VirtPageNum  {
    fn from(addr: u64) -> Self {
        VirtPageNum(addr as usize)
    }
}

impl From<u64> for VirtAddr  {
    fn from(addr: u64) -> Self {
        VirtAddr(addr as usize)
    }
}

// 实现从u64转换
impl From<u32> for PhysAddr  {
    fn from(addr: u32) -> Self {
        PhysAddr(addr as usize)
    }
}

impl From<u32> for PhysPageNum  {
    fn from(addr: u32) -> Self {
        PhysPageNum(addr as usize)
    }
}

impl From<u32> for VirtPageNum  {
    fn from(addr: u32) -> Self {
        VirtPageNum(addr as usize)
    }
}

impl From<u32> for VirtAddr  {
    fn from(addr: u32) -> Self {
        VirtAddr(addr as usize)
    }
}

// 实现转换到usize
impl From<PhysAddr> for usize  {
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

impl From<PhysPageNum> for usize  {
    fn from(addr: PhysPageNum) -> Self {
        addr.0
    }
}

impl From<VirtPageNum> for usize  {
    fn from(addr: VirtPageNum) -> Self {
        addr.0
    }
}

impl From<VirtAddr> for usize  {
    fn from(addr: VirtAddr) -> Self {
        addr.0
    }
}

// 添加debug
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PhysPageNum: {:#x}", self.0))
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PhysAddr: {:#x}", self.0))
    }
}

impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VirtPageNum: {:#x}", self.0))
    }
}

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VirtAddr: {:#x}", self.0))
    }
}

// From
impl From<PhysPageNum> for PhysAddr  {
    fn from(page: PhysPageNum) -> Self {
        PhysAddr(page.0 << 12)
    }
}

impl From<PhysAddr> for PhysPageNum  {
    fn from(page: PhysAddr) -> Self {
        PhysPageNum(page.0 >> 12)
    }
}

impl From<VirtPageNum> for VirtAddr  {
    fn from(page: VirtPageNum) -> Self {
        VirtAddr(page.0 << 12)
    }
}

impl From<VirtAddr> for VirtPageNum  {
    fn from(page: VirtAddr) -> Self {
        VirtPageNum(page.0 >> 12)
    }
}

impl Add for PhysAddr {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Add for PhysPageNum {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Add for VirtAddr {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl Add for VirtPageNum {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

// 获取原始指针
impl VirtAddr {
    pub fn as_ptr(&self) -> *const u8 {
        self.0 as *const u8
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0 as *mut u8
    }
    
    #[inline]
    // 判断地址是否有效
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl PhysAddr {
    pub fn as_ptr(&self) -> *const u8 {
        self.0 as *const u8
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0 as *mut u8
    }

    pub fn tranfer<'a, T>(&self) -> &'a mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }

    pub fn transfer_vec_count<T: Clone>(&self, count: usize) -> Vec<T> {
        let mut arr = vec![];
        for i in 0..count {
            arr.push(PhysAddr::from(self.0 + i * size_of::<T>()).tranfer::<T>().clone());
        }
        arr
    }
}

impl PhysPageNum {
    pub fn to_addr(&self) -> PhysAddr {
        PhysAddr(self.0 << 12)
    }
}

// 获取页表偏移
impl VirtAddr{
    // 页内偏移
    pub fn page_offset(&self) -> usize {
        self.0 & 0xfff
    }

    // 第一级页表偏移
    pub fn l2(&self) -> usize {
        (self.0 >> 30) & 0x1ff
    }

    // 第二级页表偏移
    pub fn l1(&self) -> usize {
        (self.0 >> 21) & 0x1ff
    }

    // 第三级页表偏移
    pub fn l0(&self) -> usize {
        (self.0 >> 12) & 0x1ff
    }
}

impl VirtPageNum {
    pub fn default() -> Self {
        Self(0)
    }

    pub fn get_l(&self, level: usize) -> usize {
        (self.0 >> (9 * level)) & 0x1ff
    }

    pub fn get_l_vec(&self) -> [usize; 3] {
        [self.get_l(2), self.get_l(1), self.get_l(0)]
    }
}

pub fn get_pages_num(size: usize) -> usize {
    (size + PAGE_SIZE - 1) / PAGE_SIZE
}

pub fn get_buf_from_phys_addr<'a>(phys_ptr: PhysAddr, size: usize) -> &'a mut[u8] {
    unsafe {
        slice::from_raw_parts_mut(usize::from(phys_ptr) as *mut u8, size)
    }
}

pub fn get_buf_from_phys_page<'a>(phys_page: PhysPageNum, pages: usize) -> &'a mut[u8] {
    get_buf_from_phys_addr(phys_page.into(), pages * PAGE_SIZE)
}

#[derive(Clone, Copy)]
pub struct UserAddr<T>(pub *mut T);

impl<T> UserAddr<T> {
    pub fn is_valid(&self) -> bool {
        self.0 as usize != 0
    }

    pub fn bits(&self) -> usize {
        self.0 as _
    }
}

impl<T> From<usize> for UserAddr<T> {
    fn from(addr: usize) -> Self {
        Self(addr as _)
    }
}