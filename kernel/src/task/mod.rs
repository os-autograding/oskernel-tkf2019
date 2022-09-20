use core::arch::global_asm;
use core::cell::RefCell;


use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;
use xmas_elf::program::{Type, SegmentData};
use crate::elf::{self, ElfExtra};
use crate::fs::filetree::INode;
use crate::memory::addr::get_pages_num;
use crate::memory::addr::get_buf_from_phys_page;
use crate::memory::mem_map::MemMap;
use crate::memory::page::alloc_more;
use crate::runtime_err::RuntimeError;
use crate::task::process::Process;
use crate::task::task_scheduler::start_tasks;
use crate::memory::page_table::PTEFlags;
use crate::memory::addr::PAGE_SIZE;
use crate::memory::addr::VirtAddr;
use crate::memory::addr::PhysAddr;
use self::task::Task;
use self::task_scheduler::NEXT_PID;

pub mod pipe;
pub mod task_queue;
pub mod stack;
pub mod controller;
pub mod pid;
pub mod process;
pub mod task;
pub mod signal;
pub mod fd_table;
pub mod task_scheduler;
pub mod user_heap;

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

// 获取pid
pub fn get_new_pid() -> usize {
    NEXT_PID.lock().next()
}

pub fn exec_with_process<'a>(process: Rc<RefCell<Process>>, task: Rc<Task>, path: &'a str, args: Vec<&'a str>) 
        -> Result<Rc<Task>, RuntimeError> {
    // 如果存在write
    let file = INode::open(None, path)?;

    let file_inner = file.0.borrow_mut();
    // 读取elf信息
    let elf = xmas_elf::ElfFile::new(&file_inner.buf).unwrap();
    let elf_header = elf.header;    
    let magic = elf_header.pt1.magic;

    let entry_point = elf.header.pt2.entry_point() as usize;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");

    // 测试代码
    let header = elf
        .program_iter()
        .find(|ph| ph.get_type() == Ok(Type::Interp));
    if let Some(header) = header {
        if let Ok(SegmentData::Undefined(_data)) = header.get_data(&elf) {
            // 对 动态链接文件进行转发
            let path = "libc.so";
            let mut new_args = vec![path];
            new_args.extend_from_slice(&args[..]);
            return exec_with_process(process, task, path, new_args);
        }
    }

    // 创建新的任务控制器 并映射栈
    let mut process = process.borrow_mut();

    let mut base = 0x20000000;
    let mut relocated_arr = vec![];

    base = match elf.relocate(base) {
        Ok(arr) => {
            relocated_arr = arr;
            base
        },
        Err(_) => {
            0
        }
    };

    // 重新映射内存 并设置头
    let mut heap_bottom = 0;
    let ph_count = elf_header.pt2.ph_count();
    for i in 0..ph_count {
        let ph = elf.program_header(i).unwrap();
        if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
            let start_va: VirtAddr = (ph.virtual_addr() as usize + base).into();
            let alloc_pages = get_pages_num(ph.mem_size() as usize + start_va.0 % 0x1000);
            let phy_start = alloc_more(alloc_pages)?;

            let ph_offset = ph.offset() as usize;
            let offset = ph.offset() as usize % PAGE_SIZE;
            let read_size = ph.file_size() as usize;
            let temp_buf = get_buf_from_phys_page(phy_start, alloc_pages);

            let vr_offset = ph.virtual_addr() as usize % 0x1000;
            let vr_offset_end = vr_offset + read_size;

            // 判断是否大于结束 修改HEAP地址
            let end_va = (((ph.virtual_addr() + ph.mem_size()) + 4095) / 4096 * 4096) as usize;
            if end_va > heap_bottom { heap_bottom = end_va; }

            // 添加memset
            process.mem_set.inner().push(MemMap::exists_page(phy_start, VirtAddr::from(ph.virtual_addr() as usize + base).into(), 
                alloc_pages, PTEFlags::VRWX | PTEFlags::U));

            // 初始化
            temp_buf[vr_offset..vr_offset_end].copy_from_slice(&file_inner.buf[ph_offset..ph_offset+read_size]);
            process.pmm.add_mapping_range(PhysAddr::from(phy_start) + PhysAddr::from(offset), 
                start_va, ph.mem_size() as usize, PTEFlags::VRWX | PTEFlags::U)?;
        }
    }
    if base > 0 {
        let pmm = process.pmm.clone();
        for (addr, value) in relocated_arr.clone() {
            let phys_addr = pmm.get_phys_addr(addr.into())?;
            let ptr = phys_addr.tranfer::<usize>();
            *ptr = value;
        }
    }

    // 添加参数
    let stack = &mut process.stack;
    let random_ptr = stack.push_arr(&[0u8; 16]);
    
    let mut auxv = BTreeMap::new();
    auxv.insert(elf::AT_PLATFORM, stack.push_str("riscv"));
    auxv.insert(elf::AT_EXECFN, stack.push_str(path));
    auxv.insert(elf::AT_PHNUM, elf_header.pt2.ph_count() as usize);
    auxv.insert(elf::AT_PAGESZ, PAGE_SIZE);
    auxv.insert(elf::AT_ENTRY, base + entry_point);
    auxv.insert(elf::AT_PHENT, elf_header.pt2.ph_entry_size() as usize);
    auxv.insert(elf::AT_PHDR, base + elf.get_ph_addr()? as usize);

    auxv.insert(elf::AT_GID, 1);
    auxv.insert(elf::AT_EGID, 1);
    auxv.insert(elf::AT_UID, 1);
    auxv.insert(elf::AT_EUID, 1);
    auxv.insert(elf::AT_SECURE, 0);
    auxv.insert(elf::AT_RANDOM, random_ptr);

    stack.init_args(args, vec![], auxv);
    
    // 更新context
    let mut task_inner = task.inner.borrow_mut();
    task_inner.context.x.fill(0);
    task_inner.context.sepc = base + entry_point;
    task_inner.context.x[2] = process.stack.get_stack_top();

    // 设置heap_bottom
    process.new_heap()?;
    process.heap.set_heap_top(heap_bottom)?;
    drop(task_inner);
    drop(process);

    // 任务管理器添加任务
    Ok(task)
}

// 执行一个程序 path: 文件名 思路：加入程序准备池  等待执行  每过一个时钟周期就执行一次
pub fn exec<'a>(path: &'a str, args: Vec<&'a str>) -> Result<Rc<Task>, RuntimeError> { 
    // 创建新的任务控制器 并映射栈
    let (process, task) = Process::new(get_new_pid(), None)?;
    exec_with_process(process, task, path, args)
}

// 包含更换任务代码
global_asm!(include_str!("change_task.asm"));

// 初始化多任务系统
pub fn init() {
    info!("多任务初始化");
    // run_first();
    start_tasks();
}