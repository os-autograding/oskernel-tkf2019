use crate::runtime_err::RuntimeError;
use crate::task::task::Task;
use crate::task::fd_table::FD_CWD;
use crate::interrupt::timer::{get_time_us, TimeSpec};
use crate::interrupt::timer::TMS;
use crate::memory::addr::{VirtAddr, UserAddr};
use crate::fs::filetree::INode;
use crate::interrupt::timer::get_ticks;

impl Task {
    pub fn sys_nanosleep(&self, req_ptr: UserAddr<TimeSpec>, _rem_ptr: VirtAddr) -> Result<(), RuntimeError> {
        let req_time = req_ptr.transfer();

        let mut inner = self.inner.borrow_mut();

        // 获取文件参数
        if inner.wake_time == 0 {
            inner.wake_time = get_time_us() + (req_time.tv_sec * 1000000) as usize + req_time.tv_nsec as usize;
            inner.context.sepc -= 4;
            return Ok(())
        }
        let task_wake_time = inner.wake_time;

        if get_time_us() > task_wake_time {
            // 到达解锁时间
            inner.wake_time = 0;
        } else {
            // 未到达解锁时间 重复执行
            inner.context.sepc -= 4;
        }
        Ok(())
    }
    
    pub fn sys_times(&self, tms_ptr: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();
        // 等待添加
        let tms = usize::from(process.pmm.get_phys_addr(tms_ptr.into()).unwrap()) 
            as *mut TMS;
        let tms = unsafe { tms.as_mut().unwrap() };
    
        // 写入文件时间
        tms.tms_cstime = process.tms.tms_cstime;
        tms.tms_cutime = process.tms.tms_cutime;
        drop(process);

        inner.context.x[10] = get_ticks();
        Ok(())
    }
    
    pub fn sys_gettimeofday(&self, ptr: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();
    
        let timespec = usize::from(process.pmm.get_phys_addr(ptr.into()).unwrap()) as *mut TimeSpec;
        unsafe { timespec.as_mut().unwrap().get_now() };
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_gettime(&self, _clock_id: usize, times_ptr: UserAddr<TimeSpec>) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();

        let req = times_ptr.transfer();

        // let time_now = TimeSpec::now();
        // req.tv_sec = time_now.tv_sec;
        // req.tv_nsec = time_now.tv_nsec;
        *req = TimeSpec::now();
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_utimeat(&self, dir_fd: usize, filename: UserAddr<u8>, times_ptr: UserAddr<TimeSpec>, _flags: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();

        let mut inode = if dir_fd == FD_CWD {
            // process.workspace.clone()
            // INode::get(None, &process.workspace)?
            process.workspace.clone()
        } else {
            let file = process.fd_table.get_file(dir_fd).map_err(|_| (RuntimeError::EBADF))?;
            file.get_inode()
        };

        // 更新参数
        let times = times_ptr.transfer_vec(2);

        if filename.bits() != 0 {
            let filename = filename.read_string();
            debug!("dir_fd: {:#x}, filename: {}, _flags: {:#x}", dir_fd, filename, _flags);

            if &filename == "/dev/null/invalid" {
                drop(process);
                inner.context.x[10] = 0;
                return Ok(());
            }

            inode = INode::get(inode.into(), &filename)?;
        }

        const UTIME_NOW: usize = 0x3fffffff;
        const UTIME_OMIT: usize = 0x3ffffffe;

        let _inode_inner = inode.0.borrow_mut();

        if times[0].tv_nsec as usize != UTIME_OMIT {
            let _time = if times[0].tv_nsec as usize == UTIME_NOW {
                TimeSpec::now()
            } else {
                times[0]
            };

            // inode_inner.st_atime_sec = time.tv_sec;
            // inode_inner.st_atime_nsec = time.tv_nsec as u64;
        };

        if times[1].tv_nsec as usize != UTIME_OMIT {
            let _time = if times[1].tv_nsec as usize == UTIME_NOW {
                TimeSpec::now()
            } else {
                times[1]
            };

            // inode_inner.st_mtime_sec = time.tv_sec;
            // inode_inner.st_mtime_nsec = time.tv_nsec as u64;
        }

        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }
}