use core::cell::RefCell;
use alloc::vec::Vec;
use alloc::rc::Rc;
use alloc::rc::Weak;
use crate::memory::page_table::PageMappingManager;
use crate::memory::mem_set::MemSet;
use crate::memory::addr::VirtAddr;
use crate::runtime_err::RuntimeError;
use crate::interrupt::timer::TMS;
use crate::fs::filetree::INode;
use super::task::Task;
use super::task::TaskStatus;
use super::stack::UserStack;
use super::fd_table::FDTable;
use super::task_scheduler::kill_process;
use super::signal::SigAction;
use super::user_heap::UserHeap;

pub struct Process {
    pub pid: usize,                             // 进程id
    pub parent: Option<Weak<RefCell<Process>>>, // 父进程
    pub pmm: Rc<PageMappingManager>,            // 内存页映射管理 
    pub mem_set: MemSet,                        // 内存使用集
    pub tasks: Vec<Weak<Task>>,                 // 任务管理器
    pub entry: VirtAddr,                        // 入口地址
    pub stack: UserStack,                       // 用户栈
    pub heap: UserHeap,                         // 用户堆
    pub workspace: Rc<INode>,                   // 工作目录
    pub fd_table: FDTable,                      // 文件描述表
    pub tms: TMS,                               // 时间记录结构
    pub sig_actions: [SigAction; 64],           // 信号结构
    pub children: Vec<Rc<RefCell<Process>>>,    // 子结构
    pub exit_code: Option<usize>                // 退出代码
}

impl Process {
    pub fn new(pid: usize, parent: Option<Weak<RefCell<Process>>>)
        -> Result<(Rc<RefCell<Process>>, Rc<Task>), RuntimeError> {
        let pmm = Rc::new(PageMappingManager::new()?);
        let heap = UserHeap::new(pmm.clone())?;
        let process = Self { 
            pid, 
            parent, 
            pmm: pmm.clone(), 
            mem_set: MemSet::new(), 
            tasks: vec![], 
            entry: 0usize.into(), 
            stack: UserStack::new(pmm.clone())?, 
            heap, 
            workspace: INode::root(),
            fd_table: FDTable::new(),
            children: vec![],
            sig_actions: [SigAction::empty(); 64],
            tms: TMS::new(),
            exit_code: None
        };
        // 创建默认任务
        let process = Rc::new(RefCell::new(process));
        // 添加到子任务
        let task = Task::new(0, process.clone());
        // process.borrow_mut().tasks.push(Rc::downgrade(&task));
        Ok((process, task))
    }

    pub fn fork(pid: usize, parent: Rc<RefCell<Process>>) -> Result<(Rc<RefCell<Process>>, Rc<Task>), RuntimeError> {
        let parent_inner = parent.borrow_mut();
        let pmm = Rc::new(PageMappingManager::new()?);
        pmm.add_mapping_by_set(&parent_inner.mem_set)?;
        pmm.add_mapping_by_set(&parent_inner.heap.mem_set)?;
        let process = Rc::new(RefCell::new(Self { 
            pid, 
            parent: Some(Rc::downgrade(&parent)), 
            pmm: pmm, 
            mem_set: parent_inner.mem_set.clone(), 
            tasks: vec![], 
            entry: parent_inner.entry, 
            stack: parent_inner.stack.clone(), 
            heap: parent_inner.heap.clone(), 
            workspace: INode::root(),
            fd_table: parent_inner.fd_table.clone(),
            children: vec![],
            sig_actions: [SigAction::empty(); 64],
            tms: TMS::new(),
            exit_code: None
        }));
        let task = Task::new(0, process.clone());
        Ok((process, task))
    }

    // 进程进行等待
    pub fn wait(&self) {
        // TODO: 进程进入等待状态  等待目标进程结束
        // let task = self.get_task(0);
        // task.inner.borrow_mut().status = TaskStatus::WAITING;
    }

    pub fn new_heap(&mut self) -> Result<(), RuntimeError>{
        self.heap = UserHeap::new(self.pmm.clone())?;
        Ok(())
    }

    // 判断是否在等待状态
    pub fn is_waiting(&self) -> bool {
        // tasks的len 一定大于 0
        let task = self.get_task(0);
        // 如果父进程在等待 则直接释放资源 并改变父进程的状态
        if task.inner.borrow().status == TaskStatus::WAITING {
            true
        } else {
            false
        }
    }

    // 获取task 任务
    pub fn get_task(&self, index: usize) -> Rc<Task> {
        if index >= self.tasks.len() {
            panic!("in process.rs index >= task.len()");
        }
        self.tasks[0].upgrade().unwrap()
    }

    // 结束进程
    pub fn exit(&mut self, exit_code: usize) {
        self.release();
        // 如果没有子进程
        self.exit_code = Some(exit_code);
        // 进程回收
        kill_process(self.pid);
    }

    // 重置内存信息
    pub fn reset(&mut self) -> Result<(), RuntimeError>{
        let pmm = Rc::new(PageMappingManager::new()?);
        let mem_set = MemSet::new();
        self.pmm = pmm;
        self.mem_set = mem_set;
        self.stack = UserStack::new(self.pmm.clone())?;
        Ok(())
    }

    // 释放内存
    pub fn release(&mut self) {
        self.stack.release();
        self.heap.mem_set.release();
        self.mem_set.release();
        self.pmm.release();
    }
}
    