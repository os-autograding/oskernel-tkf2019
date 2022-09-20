use alloc::{vec::Vec, collections::VecDeque};
use k210_soc::sleep::usleep;

use crate::sync::mutex::Mutex;
use crate::memory::page::get_free_page_num;
use crate::task::task_scheduler::add_task_to_scheduler;


use super::exec;

lazy_static! {
    pub static ref TASK_QUEUE: Mutex<VecDeque<&'static str>> = Mutex::new(VecDeque::from(vec![
        // 调试信息
        "busybox sh busybox_testcode.sh",
        "busybox sh test.sh date.lua",
        "busybox sh test.sh file_io.lua",
        "busybox sh test.sh max_min.lua",
        "busybox sh test.sh random.lua",
        "busybox sh test.sh remove.lua",
        "busybox sh test.sh round_num.lua",
        "busybox sh test.sh sin30.lua",
        "busybox sh test.sh sort.lua",
        "busybox sh test.sh strings.lua",
        "busybox sh run-dynamic.sh"
        
        // // lmbench_all
        // "busybox mkdir -p /var/tmp",
        // "busybox touch /var/tmp/lmbench",
        // "busybox cp hello /tmp",

        // latency measurements
        // "lmbench_all lat_syscall -P 1 null",
        // "lmbench_all lat_syscall -P 1 read",
        // "lmbench_all lat_syscall -P 1 write",
        // "lmbench_all lat_syscall -P 1 stat /var/tmp/lmbench",
        // "lmbench_all lat_syscall -P 1 fstat /var/tmp/lmbench",
        // "lmbench_all lat_syscall -P 1 open /var/tmp/lmbench",
        // "lmbench_all lat_select -n 100 -P 1 file",
        // "lmbench_all lat_sig -P 1 install",
        // "lmbench_all lat_sig -P 1 catch",
        // "lmbench_all lat_proc -P 1 fork",
        // "lmbench_all lat_proc -P 1 exec",
        // "lmbench_all lat_proc -P 1 shell",
        // "lmbench_all lat_mmap -P 1 512k /var/tmp/XXX",
        // "lmbench_all bw_file_rd -P 1 512k io_only /var/tmp/XXX",
        // "lmbench_all bw_file_rd -P 1 512k open2close /var/tmp/XXX",
        // "lmbench_all bw_mmap_rd -P 1 512k mmap_only /var/tmp/XXX",
        // "lmbench_all bw_mmap_rd -P 1 512k open2close /var/tmp/XXX",
        // "lmbench_all lat_ctx -P 1 -s 32 2 4 8 16 24 32 64 96",  // 最后执行 可能出现问题

        // "lmbench_all lat_sig -P 1 prot lat_sig", // 暂时出问题
        // "lmbench_all lat_pipe -P 1", // 暂时出问题
        // "lmbench_all lmdd label=\"File /var/tmp/XXX write bandwidth:\" of=/var/tmp/XXX move=1m fsync=1 print=3", // 暂时出问题
        // "lmbench_all lat_pagefault -P 1 /var/tmp/XXX",  // 暂时出问题

        // file system latency
        // "lmbench_all lat_fs /var/tmp",   // 暂时出问题
        
        // Bandwidth measurements
        // "lmbench_all bw_pipe -P 1",      // 暂时出问题

    ]));
}

pub static mut LAST_PRO: &str = "";

pub fn exec_by_str(str: &'static str) {
    debug!("执行任务: {}", str);
    let args: Vec<&str> = str.split(" ").collect();
    if unsafe {LAST_PRO} == args[0] {
        #[cfg(feature = "board_k210")]
        usleep(200000);
        unsafe { LAST_PRO = args[0]}
    }
    if let Ok(task) = exec(args[0], args[0..].to_vec()) {
        task.before_run();
        add_task_to_scheduler(task);
    }
}


// 加载下一个任务
pub fn load_next_task() -> bool {
    if let Some(pro_name) = TASK_QUEUE.lock().pop_front() {
        info!("剩余页表: {}", get_free_page_num());
        exec_by_str(pro_name);
        true
    } else {
        info!("剩余页表: {}", get_free_page_num());
        false
    }
}
