
use virtio_drivers::{Hal, PhysAddr, VirtAddr};

use crate::memory::{page::PAGE_ALLOCATOR, addr::PhysPageNum};


pub struct HalImpl;

impl Hal for HalImpl {
    fn dma_alloc(pages: usize) -> PhysAddr {
        info!("申请设备地址!");
        if let Ok(page_num) = PAGE_ALLOCATOR.lock().alloc_more(pages) {
            return usize::from(page_num) << 12
        } else {
            panic!("申请失败");
        }
    }

    fn dma_dealloc(paddr: PhysAddr, pages: usize) -> i32 {
        PAGE_ALLOCATOR.lock().dealloc_more(PhysPageNum::from(paddr), pages);
        0
    }

    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        VirtAddr::from(usize::from(paddr))
    }

    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        vaddr
    }
}