use core::arch::asm;

use alloc::collections::VecDeque;
use alloc::rc::Rc;
use crate::sync::mutex::Mutex;
use crate::sys_call::is_vfork_wait;
use crate::task::pid::PidGenerater;
use crate::interrupt::timer::task_time_refresh;
use crate::memory::page_table::switch_to_kernel_page;
use super::task::Task;
use super::task::TaskStatus;
use super::task_queue::load_next_task;

// 任务控制器管理器
pub struct TaskScheduler {
    pub queue: VecDeque<Rc<Task>>,          // 准备队列
    pub is_run: bool                    // 任务运行标志
}

impl TaskScheduler {
    // 创建Task调度器
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            is_run: false
        }
    }

    // 添加任务调度器
    pub fn add_task(&mut self, task: Rc<Task>) {
        self.queue.push_back(task.clone());
    }

    // 执行下一个任务
    pub fn switch_next(&mut self) {
        if let Some(task) = self.queue.pop_front() {
            // task.before_run();
            task.inner.borrow_mut().status = TaskStatus::READY;
            self.queue.push_back(task);
            self.queue[0].before_run();
        }
        task_time_refresh();     
    }

    // 执行第一个任务
    /// 进行调度更新
    pub fn start(&mut self) {
        info!("开始执行任务");
        loop {
            // 没有任务时从任务队列取出任务
            if self.queue.len() == 0 {
                if !load_next_task() {
                    break;
                }
                unsafe {
                    // 为什么需要缓冲？
                    for i in 0..10 {
                        asm!("nop");
                    }
                }
            }
            // TODO: 判断是否存在等待中的任务 如果存在就切换任务
            let task = self.queue[0].clone();
            if is_vfork_wait(task.pid) {
                switch_next();
                continue;
            }
            self.is_run = true;
            warn!("执行pid: {}   tid: {}   tasks len: {}", task.pid, task.tid, self.queue.len());
            task.run();
            task.catch();
        }
    }

    // 关闭进程
    pub fn kill_process(&mut self, pid: usize) {
        self.queue = self.queue.clone().into_iter().filter(|x| x.pid != pid).collect();
    }

    // 关闭进程
    pub fn kill_task(&mut self, pid: usize, tid: usize) {
        self.queue = self.queue.clone().into_iter().filter(|x| x.pid != pid || x.tid != tid).collect();
    }

}

lazy_static! {
    // 任务管理器和pid生成器
    pub static ref TASK_SCHEDULER: Mutex<TaskScheduler> = Mutex::new(TaskScheduler::new());
    pub static ref NEXT_PID: Mutex<PidGenerater> = Mutex::new(PidGenerater::new());
}

pub fn start_tasks() {
    // 刷新下一个调度时间
    // info!("开始任务");
    task_time_refresh();
    let mut task_scheduler = TASK_SCHEDULER.force_get();
    task_scheduler.start();
    switch_to_kernel_page();
    // 切换到内核页表
}

pub fn add_task_to_scheduler(task: Rc<Task>) {
    TASK_SCHEDULER.force_get().add_task(task);
}

pub fn kill_process(pid: usize) {
    TASK_SCHEDULER.force_get().kill_process(pid);
}

pub fn kill_task(pid: usize, tid: usize) {
    TASK_SCHEDULER.force_get().kill_task(pid, tid);
}

pub fn switch_next() {
    TASK_SCHEDULER.force_get().switch_next();
}

pub fn get_current_task() -> Option<Rc<Task>> {
    match TASK_SCHEDULER.force_get().queue.front() {
        Some(task) => Some(task.clone()),
        None => None
    }
}

pub fn get_task(pid: usize, tid: usize) -> Option<Rc<Task>> {
    let task_scheduler = TASK_SCHEDULER.force_get();
    for i in 0..task_scheduler.queue.len() {
        let task = task_scheduler.queue[i].clone();
        if task.pid == pid && task.tid == tid {
            return Some(task.clone());
        }
    }
    None
}

pub fn switch_to_task(pid: usize, tid: usize) {
    let mut task_scheduler = TASK_SCHEDULER.force_get();

    while let Some(task) = task_scheduler.queue.pop_front() {
        let ctask = task.clone();
        task_scheduler.queue.push_back(task);
        if ctask.tid == tid && ctask.pid == pid {
            break;
        }
    }
}

// 获取当前的任务数量
pub fn get_task_num() -> usize {
    TASK_SCHEDULER.force_get().queue.len()
}