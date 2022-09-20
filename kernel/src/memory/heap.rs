
use buddy_system_allocator::LockedHeap;

// 堆大小
const HEAP_SIZE: usize = 0x0008_0000;

// 堆空间
static mut HEAP: [u8;HEAP_SIZE] = [0;HEAP_SIZE];

// 堆内存分配器
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<64> = LockedHeap::empty();

#[cfg(not(feature = "board_k210"))]
const PROGRAM_START:usize = 0x80200000;

#[cfg(feature = "board_k210")]
const PROGRAM_START:usize = 0x80020000;

// 初始化堆内存分配器
pub fn init() {
    extern "C" {
        fn end();
        fn stext();
        fn etext();
    }
    unsafe {
        HEAP_ALLOCATOR.lock().init(HEAP.as_ptr() as usize, HEAP_SIZE);
        let file_size = end as usize - PROGRAM_START;

        let file_size_kb = file_size / 1024;

        let text_start = stext as usize;
        let text_end = etext as usize;
        info!("程序大小为: {} kb  堆大小: {} kb  代码段: {} kb", file_size_kb, HEAP_SIZE / 1024, (text_end - text_start)/1024);
    }
}