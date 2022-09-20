use core::arch::asm;

mod heap;
pub mod page;
pub mod addr;
pub mod page_table;
pub mod mem_map;
pub mod mem_set;

pub const KERNEL_STACK_SIZE: usize = 4096;

lazy_static! {
    static ref KERNEL_STACK:[u8; KERNEL_STACK_SIZE] = [0u8; KERNEL_STACK_SIZE];
}

// 内存初始化
pub fn init() {
    // 初始化内核栈
    let kernel_stack_top = KERNEL_STACK.as_ptr() as usize + KERNEL_STACK_SIZE;
    unsafe {
        asm!("csrw sscratch, a0", in("a0") kernel_stack_top);
    }
    // 初始化堆 便于变量指针分配
    heap::init();

    // 初始化页管理器
    page::init();

    // 开始页映射
    page_table::init();
}