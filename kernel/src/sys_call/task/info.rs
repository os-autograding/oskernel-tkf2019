use crate::{runtime_err::RuntimeError, sys_call::{SYS_CALL_ERR, UTSname}, task::task::{Task, Rusage}, memory::addr::UserAddr, interrupt::timer::TimeSpec};

impl Task {
    // 获取系统信息
    pub fn sys_uname(&self, ptr: UserAddr<UTSname>) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
    
        // 获取参数
        let sys_info = ptr.transfer();
        // 写入系统信息

        // let sys_name = b"ByteOS";
        // let sys_nodename = b"ByteOS";
        // let sys_release = b"release";
        // let sys_version = b"alpha 1.1";
        // let sys_machine = b"riscv k210";
        // let sys_domain = b"alexbd.cn";
        let sys_name = b"Linux";
        let sys_nodename = b"debian";
        let sys_release = b"5.10.0-7-riscv64";
        let sys_version = b"#1 SMP Debian 5.10.40-1 (2021-05-28)";
        let sys_machine = b"riscv k210";
        let sys_domain = b"alexbd.cn";

        sys_info.sysname[..sys_name.len()].copy_from_slice(sys_name);
        sys_info.nodename[..sys_nodename.len()].copy_from_slice(sys_nodename);
        sys_info.release[..sys_release.len()].copy_from_slice(sys_release);
        sys_info.version[..sys_version.len()].copy_from_slice(sys_version);
        sys_info.machine[..sys_machine.len()].copy_from_slice(sys_machine);
        sys_info.domainname[..sys_domain.len()].copy_from_slice(sys_domain);
        inner.context.x[10] = 0;
        Ok(())
    }
    
    // 获取pid
    pub fn sys_getpid(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = self.pid;
        Ok(())
    }
    
    // 获取父id
    pub fn sys_getppid(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.clone();
        let process = process.borrow();

        inner.context.x[10] = match &process.parent {
            Some(parent) => {
                let parent = parent.upgrade().unwrap();
                let x = parent.borrow().pid; 
                x
            },
            None => SYS_CALL_ERR
        };

        Ok(())
    }
    
    // 获取线程id
    pub fn sys_gettid(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = self.tid;
        Ok(())
    }

    pub fn sys_getrusage(&self, _who: usize, usage: UserAddr<Rusage>) -> Result<(), RuntimeError>{
        let mut inner = self.inner.borrow_mut();
        let usage = usage.transfer();
        usage.ru_stime = TimeSpec::now();
        usage.ru_utime = TimeSpec::now();
        inner.context.x[10] = SYS_CALL_ERR;
        Ok(())
    }

    // 设置 tid addr
    pub fn sys_set_tid_address(&self, tid_ptr: UserAddr<u32>) -> Result<(), RuntimeError> {
        // 测试写入用户空间
        let tid_ptr = tid_ptr.transfer();
        let mut inner = self.inner.borrow_mut();
        let clear_child_tid = self.clear_child_tid.borrow().clone();

        *tid_ptr = if clear_child_tid.is_valid() {
            clear_child_tid.transfer().clone()
        } else {
            0
        };

        inner.context.x[10] = self.tid;
        Ok(())
    }    
}