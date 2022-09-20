use crate::{task::{task::Task, task_scheduler::get_task}, runtime_err::RuntimeError, sys_call::{remove_vfork_wait, SYS_CALL_ERR}, memory::page::get_free_page_num};

impl Task {
    /// 退出当前任务 
    pub fn sys_exit(&self, exit_code: usize) -> Result<(), RuntimeError> {
        let inner = self.inner.borrow();
        if self.tid == 0 {
            inner.process.borrow_mut().exit(exit_code);
        } else {
            self.exit();
        }

        let clear_child_tid = self.clear_child_tid.borrow().clone();
        if clear_child_tid.is_valid() {
            *clear_child_tid.transfer() = 0;
        }
        Err(RuntimeError::KillCurrentTask)
    }
    
    // 退出当前进程？ eg: 功能也许有待完善
    pub fn sys_exit_group(&self, exit_code: usize) -> Result<(), RuntimeError> {
        let inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        debug!("exit pid: {}", self.pid);
        process.exit(exit_code);
        match &process.parent {
            Some(parent) => {
                let parent = parent.upgrade().unwrap();
                let parent = parent.borrow();
                remove_vfork_wait(parent.pid);

                // let end: UserAddr<TimeSpec> = 0x10bb78.into();
                // let start: UserAddr<TimeSpec> = 0x10bad0.into();

                // println!("start: {:?}   end: {:?}",start.transfer(), end.transfer());

                // let target_end: UserAddr<TimeSpec> = parent.pmm.get_phys_addr(0x10bb78usize.into())?.0.into();
                // let target_start: UserAddr<TimeSpec> = parent.pmm.get_phys_addr(0x10bad0usize.into())?.0.into();
                // *target_start.transfer() = *start.transfer();
                // *target_end.transfer() = *end.transfer();

                // let task = parent.tasks[0].clone().upgrade().unwrap();
                // drop(parent);
                // // 处理signal 17 SIGCHLD
                // task.signal(17);
            }
            None => {}
        }
        debug!("剩余页表: {}", get_free_page_num());
        debug!("exit_code: {:#x}", exit_code);
        Err(RuntimeError::ChangeTask)
    }

    // kill task
    pub fn sys_kill(&self, _pid: usize, _signum: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        debug!(
            "kill: thread {} kill process {} with signal {:?}",
            0,
            _pid,
            _signum
        );

        inner.context.x[10] = 0;
        Ok(())
    }

    // kill task
    pub fn sys_tkill(&self, tid: usize, signum: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        let signal_task = get_task(self.pid, tid);
        debug!("signum: {}", signum);
        if let Some(signal_task) = signal_task {
            drop(inner);
            signal_task.signal(signum)?;
        }
        Ok(())
    }

    pub fn sys_tgkill(&self, tgid: usize, tid: usize, signum: usize) -> Result<(), RuntimeError> {
        debug!("tgkill: tgid: {}  tid: {}  signum {}", tgid, tid, signum);
        if let Some(task) = get_task(tgid, tid) {
            task.signal(signum)?;
        } else {
            self.update_context(|x| x.x[10] = SYS_CALL_ERR);
        }
        Ok(())
    }
    
}