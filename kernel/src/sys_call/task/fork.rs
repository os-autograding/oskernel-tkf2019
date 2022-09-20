use alloc::{string::String, vec::Vec, rc::Rc};
use k210_pac::uart1::tar;

use crate::{runtime_err::RuntimeError, sys_call::{SYS_CALL_ERR, CloneFlags, add_vfork_wait}, memory::{addr::UserAddr, page_table::switch_to_kernel_page}, task::{exec_with_process, task_scheduler::{get_task_num, add_task_to_scheduler}, task::{Task, TaskStatus}, pid::get_next_pid, process::Process}};

impl Task {

    pub fn sys_sched_yield(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.status = TaskStatus::READY;
        Err(RuntimeError::ChangeTask)
    }
    
    // fork process
    pub fn sys_fork(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.clone();
        let mut process = process.borrow_mut();

        let (child_process, child_task) =
            Process::new(get_next_pid(), Some(Rc::downgrade(&inner.process)))?;
        process.children.push(child_process.clone());
        let mut child_task_inner = child_task.inner.borrow_mut();
        child_task_inner.context.clone_from(&inner.context);
        child_task_inner.context.x[10] = 0;
        drop(child_task_inner);
        add_task_to_scheduler(child_task.clone());
        let cpid = child_task.pid;
        inner.context.x[10] = cpid;
        let mut child_process = child_process.borrow_mut();
        child_process.mem_set = process.mem_set.clone_with_data()?;
        child_process.stack = process.stack.clone_with_data(child_process.pmm.clone())?;
        // 复制fd_table
        child_process.fd_table = process.fd_table.clone();
        // 创建新的heap
        // child_process.heap = UserHeap::new(child_process.pmm.clone())?;
        child_process.heap = process.heap.clone_with_data(child_process.pmm.clone())?;
        debug!("heap_pointer: {:#x}", child_process.heap.get_heap_top());
        child_process.pmm.add_mapping_by_set(&child_process.mem_set)?;
        drop(process);
        drop(child_process);
        drop(inner);
        // Ok(())
        Err(RuntimeError::ChangeTask)
    }

    pub fn sys_spec_fork(&self, flags: usize, _new_sp: usize, _ptid: UserAddr<u32>, _tls: usize, ctid_ptr: UserAddr<u32>) -> Result<(), RuntimeError>{
        // return self.sys_fork();
        let flags = CloneFlags::from_bits_truncate(flags);
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.clone();

        let cpid = get_next_pid();
        let (child_process, child_task) =
            Process::fork(cpid, process.clone())?;
        
        let mut process = process.borrow_mut();
        process.children.push(child_process.clone());

        let mut child_task_inner = child_task.inner.borrow_mut();
        child_task_inner.context.clone_from(&inner.context);
        child_task_inner.context.x[10] = 0;
        drop(child_task_inner);

        add_task_to_scheduler(child_task.clone());

        let mut child_process = child_process.borrow_mut();
        child_process.stack = process.stack.clone_with_data(child_process.pmm.clone())?;
        // child_process.heap = process.heap.clone_with_data(child_process.pmm.clone())?;
        inner.context.x[10] = cpid;

        if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
            *ctid_ptr.transfer() = cpid as u32;
        }

        if flags.contains(CloneFlags::CLONE_CHILD_CLEARTID) {
            
        }

        drop(process);
        drop(child_process);
        drop(inner);
        // Ok(())
        add_vfork_wait(self.pid);
        Err(RuntimeError::ChangeTask)
    }
    
    // clone task
    pub fn sys_clone(&self, flags: usize, new_sp: usize, ptid: UserAddr<u32>, tls: usize, ctid_ptr: UserAddr<u32>) -> Result<(), RuntimeError> {
        // let flags = flags & 0x4fff;
        debug!(
            "clone: flags={:#x}, newsp={:#x}, parent_tid={:#x}, child_tid={:#x}, newtls={:#x}",
            flags, new_sp, ptid.bits(), ctid_ptr.0 as usize, tls
        );
        if flags == 0x4111 || flags == 0x11 {
            // VFORK | VM | SIGCHILD
            warn!("sys_clone is calling sys_fork instead, ignoring other args");
            return self.sys_fork();
        } else if flags == 0x1200011 {
            return self.sys_spec_fork(0x11, new_sp, ptid, tls, ctid_ptr);
            // return self.sys_fork();
        }

        debug!(
            "clone: flags={:#x}, newsp={:#x}, parent_tid={:#x}, child_tid={:#x}, newtls={:#x}",
            flags, new_sp, ptid.bits(), ctid_ptr.0 as usize, tls
        );

        let mut inner = self.inner.borrow_mut();
        let process = inner.process.clone();
        let process = process.borrow();
        
        let ctid = process.tasks.len();
        drop(process);

        let new_task = Task::new(ctid, inner.process.clone());
        let mut new_task_inner = new_task.inner.borrow_mut();
        new_task_inner.context.clone_from(&inner.context);
        new_task_inner.context.x[2] = new_sp;
        new_task_inner.context.x[4] = tls;
        new_task_inner.context.x[10] = 0;
        add_task_to_scheduler(new_task.clone());
        // 添加到process
        inner.context.x[10] = ctid;
        
        debug!("tasks: len {}", get_task_num());

        drop(new_task_inner);
        drop(inner);
        if ptid.is_valid() {
            *ptid.transfer() = ctid as u32;
        }
        // 执行 set_tid_address
        new_task.set_tid_address(ctid_ptr);
        // just finish clone, not change task
        Ok(())
        // Err(RuntimeError::ChangeTask)
    }

    // 执行文件
    pub fn sys_execve(&self, filename: UserAddr<u8>, argv: UserAddr<UserAddr<u8>>, 
            _envp: UserAddr<UserAddr<u8>>) -> Result<(), RuntimeError> {
        let inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        let filename = filename.read_string();

        debug!("run {}", filename);
        let args = argv.transfer_until(|x| !x.is_valid());
        let args:Vec<String> = args.iter_mut().map(|x| x.read_string()).collect();

        // 读取envp
        // let envp = argv.translate_until(pmm.clone(), |x| !x.is_valid());
        // let envp:Vec<String> = envp.iter_mut().map(|x| x.read_string(pmm.clone())).collect();

        // 获取 envp
        let task = process.tasks[self.tid].clone().upgrade().unwrap();
        process.reset()?;
        drop(process);
        let process = inner.process.clone();
        drop(inner);
        switch_to_kernel_page();
        exec_with_process(process.clone(), task, &filename, args.iter().map(AsRef::as_ref).collect())?;
        // process.borrow_mut().new_heap()?;
        self.before_run();
        Ok(())
    }
    
    // wait task
    pub fn sys_wait4(&self, pid: usize, ptr: UserAddr<i32>, _options: usize) -> Result<(), RuntimeError> {
        debug!("pid: {:#x}, ptr: {:#x}, _options: {}", pid, ptr.bits(), _options);
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.clone();
        let mut process = process.borrow_mut();


        if pid != SYS_CALL_ERR {
            // let target = 
            // process.children.iter().find(|&x| x.borrow().pid == pid);

            // if let Some(exit_code) = target.map_or(None, |x| x.borrow().exit_code) {
            //     if ptr.is_valid() {
            //         *ptr.transfer() = exit_code as i32;
            //     }

            //     inner.context.x[10] = pid;
            //     return Ok(())
            // }

            
            let target = 
                process.children.iter().find(|&x| x.borrow().pid == pid);

            if let Some(target) = target {
                let target = target.borrow();
                if let Some(exit_code) = target.exit_code {
                    if ptr.is_valid() {
                        *ptr.transfer() = exit_code as i32;
                    }

                    debug!("hava task");
                    
                    let t_pid = target.pid;
                    drop(target);
                    process.children.drain_filter(|x| x.borrow().pid == t_pid);

                    inner.context.x[10] = pid;
                    return Ok(())
                }
            } else {
                debug!("not hava task");
                inner.context.x[10] = -10 as isize as usize;
                return Ok(());
            }

            // if let Some(exit_code) = target.map_or(None, |x| x.borrow().exit_code) {
            //     if ptr.is_valid() {
            //         *ptr.transfer() = exit_code as i32;
            //     }

            //     inner.context.x[10] = pid;
            //     return Ok(())
            // }
        } else {
            if process.children.len() == 0 {
                inner.context.x[10] = -10 as isize as usize;
                return Ok(());
            }

            let cprocess_vec = 
                process.children.drain_filter(|x| x.borrow().exit_code.is_some()).collect::<Vec<_>>();

            debug!("cpro len: {}", cprocess_vec.len());

            if cprocess_vec.len() != 0 {
                let cprocess = cprocess_vec[0].borrow();
                if ptr.is_valid() {
                    *ptr.transfer() = cprocess.exit_code.unwrap() as i32;
                }
                inner.context.x[10] = cprocess.pid;
                return Ok(());
            }
        }
        inner.context.sepc -= 4;
        drop(process);
        drop(inner);
        Err(RuntimeError::ChangeTask)
    }
}