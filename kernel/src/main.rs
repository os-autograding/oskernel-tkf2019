// remove std lib
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![allow(unaligned_references)]
#![feature(const_btree_new)]
#![feature(drain_filter)]


// 使用定义的命令行宏   
#[macro_use]
mod console;
mod device;
pub mod interrupt;
mod memory;
mod fs;
mod sbi;
mod panic;
mod sync;
pub mod task;
pub mod runtime_err;
pub mod elf;
pub mod sys_call;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static; 
#[macro_use]
extern crate alloc;
use core::arch::global_asm;


use alloc::{rc::Rc, string::ToString};
use riscv::register::sstatus;
use crate::fs::filetree::INode;
use crate::fs::filetree::DiskFileEnum;
use crate::fs::file::FileType;
use crate::fs::cache::cache_file;
use crate::memory::page::get_free_page_num;
mod virtio_impl;


global_asm!(include_str!("entry.asm"));

/// 清空bss段
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_start_addr = sbss as usize as *mut u8;
    let bss_size = ebss as usize - sbss as usize;
    unsafe {
        core::slice::from_raw_parts_mut(bss_start_addr, bss_size).fill(0);
    }
    
    // 显示BSS段信息
    info!("the bss section range: {:X}-{:X}, {} KB", sbss as usize, ebss as usize, bss_size / 0x1000);
}

#[no_mangle]
pub extern "C" fn rust_main(hart_id: usize, device_tree_p_addr: usize) -> ! {
    // // 保证仅有一个核心工作
    #[cfg(not(debug_assertions))]
    if hart_id != 0 {
        sbi::hart_suspend(0x00000000, support_hart_resume as usize, 0);
    }

    unsafe {
        sstatus::set_fs(sstatus::FS::Dirty);
    }
    
    // 清空bss段
    clear_bss();

    // 输出设备信息
    info!("当前核心 {}", hart_id);
    info!("设备树地址 {:#x}", device_tree_p_addr);

    // 提示信息
    info!("Welcome to test os!");

    // 开启SUM位 让内核可以访问用户空间  踩坑：  
    // only in qemu. eg: qemu is riscv 1.10    k210 is riscv 1.9.1  
    // in 1.10 is SUM but in 1.9.1 is PUM which is the opposite meaning with SUM
    #[cfg(not(feature = "board_k210"))]
    unsafe {
        sstatus::set_sum();
    }

    // 初始化内存
    memory::init();

    // 初始化中断
    interrupt::init();

    // 初始化设备
    device::init();

    // 初始化文件系统
    fs::init();

    // 输出文件树
    print_file_tree(INode::root());

    // 创建busybox 指令副本
    let busybox_node = INode::get(None, "busybox").expect("can't find busybox");
    busybox_node.linkat("sh");
    busybox_node.linkat("echo");
    busybox_node.linkat("cat");
    busybox_node.linkat("cp");
    busybox_node.linkat("ls");
    busybox_node.linkat("pwd");
    INode::mkdir(None, "/bin", 0).expect("can't create bin directory");
    INode::mkdir(None, "/sbin", 0).expect("can't create sbin directory");
    busybox_node.linkat("bin/busybox");
    // let lmbench_all = INode::get(None, "lmbench_all").expect("can't find busybox");
    // lmbench_all.linkat("sbin/lmbench_all");
    // lmbench_all.linkat("bin/lmbench_all");
    // // let lmbench_all = INode::get(None, "busybox_cmd.txt").expect("can't find busybox");
    // lmbench_all.linkat("var/tmp/XXX");

    INode::root().add(INode::new("proc".to_string(), 
        DiskFileEnum::None, FileType::Directory, None));

    #[cfg(not(feature = "board_k210"))]
    {
        // 非k210缓冲文件
        cache_file("busybox");
        cache_file("lua");
        // cache_file("lmbench_all");
    }

    // 初始化多任务
    task::init();

    // 输出剩余页表
    debug!("剩余页表: {}", get_free_page_num());

    // 调用rust api关机
    panic!("正常关机")
}

#[allow(unused)]
/// 暂时不使用  目前只使用单核
extern "C" fn support_hart_resume(hart_id: usize, _param: usize) {
    info!("核心 {} 作为辅助核心进行等待", hart_id);
    loop {} // 进入循环
}


// 打印目录树
pub fn print_file_tree(node: Rc<INode>) {
    info!("{}", node.get_pwd());
    print_file_tree_back(node, 0);
}

// 打印目录树 - 递归
pub fn print_file_tree_back(node: Rc<INode>, space: usize) {
    let iter = node.clone_children();
    let mut iter = iter.iter().peekable();
    while let Some(sub_node) = iter.next() {
        if iter.peek().is_none() {
            info!("{:>2$}└──{}", "", sub_node.get_filename(), space);
        } else {
            info!("{:>2$}├──{}", "", sub_node.get_filename(), space);
        }
        if sub_node.is_dir() {
            print_file_tree_back(sub_node.clone(), space + 3);
        }
    }
}