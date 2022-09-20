use alloc::{rc::Rc, string::ToString};

use crate::{task::{task::Task, fd_table::{FileDesc, FD_NULL}, pipe::new_pipe}, runtime_err::RuntimeError, memory::addr::UserAddr, sys_call::OpenFlags, fs::{stdio::{StdZero, StdNull}, specials::{proc_mounts::ProcMounts, proc_meminfo::ProcMeminfo, etc_adjtime::EtcAdjtime, dev_rtc::DevRtc}, filetree::INode}, interrupt::timer::TimeSpec};

impl Task {
    // 复制文件描述符
    pub fn sys_dup(&self, fd: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        let fd_v = process.fd_table.get(fd)?.clone();
        // 判断文件描述符是否存在
        let new_fd = process.fd_table.push(fd_v);
        drop(process);
        inner.context.x[10] = new_fd;
        Ok(())
    }
    // 复制文件描述符
    pub fn sys_dup3(&self, fd: usize, new_fd: usize) -> Result<(), RuntimeError> {
        debug!("dup fd: {} to fd: {}", fd, new_fd);
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        // 判断是否存在文件描述符
        let fd_v = process.fd_table.get(fd)?.clone();
        // if let Ok(file) = fd_v.clone().downcast::<File>() {
        //     file.lseek(0, 0);
        // }
        process.fd_table.set(new_fd, fd_v);
        drop(process);
        inner.context.x[10] = new_fd;
        Ok(())
    }
    // 打开文件
    pub fn sys_openat(&self, fd: usize, filename: UserAddr<u8>, flags: usize, _open_mod: usize) -> Result<(), RuntimeError> {
        let filename = filename.read_string();
        debug!("open file: {}  flags: {:#x}", filename, flags);
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();

        // 获取文件信息
        let flags = OpenFlags::from_bits_truncate(flags as u32);

        if filename == "/dev/zero" {
            let fd = process.fd_table.push(FileDesc::new(Rc::new(StdZero)));
            drop(process);
            inner.context.x[10] = fd;
            return Ok(())
        } else if filename == "/dev/null" {
            let fd = process.fd_table.push(FileDesc::new(Rc::new(StdNull)));
            drop(process);
            inner.context.x[10] = fd;
            return Ok(())
        } else if filename == "/proc/mounts" {
            let fd = process.fd_table.push(FileDesc::new(Rc::new(ProcMounts::new())));
            drop(process);
            inner.context.x[10] = fd;
            return Ok(())
        } else if filename == "/proc/meminfo" {
            let fd = process.fd_table.push(FileDesc::new(Rc::new(ProcMeminfo::new())));
            drop(process);
            inner.context.x[10] = fd;
            return Ok(())
        } else if filename == "/etc/adjtime" {
            let fd = process.fd_table.push(FileDesc::new(Rc::new(EtcAdjtime::new())));
            drop(process);
            inner.context.x[10] = fd;
            return Ok(())
        } else if filename == "/dev/rtc" {
            let fd = process.fd_table.push(FileDesc::new(Rc::new(DevRtc::new())));
            drop(process);
            inner.context.x[10] = fd;
            return Ok(())
        }


        // 判断文件描述符是否存在
        let current = if fd == FD_NULL {
            None
        } else {
            let file = process.fd_table.get_file(fd)?;
            Some(file.get_inode())
        };
        // 根据文件类型匹配
        let file = if flags.contains(OpenFlags::CREATE) {
            INode::open_or_create(current, &filename)?
        } else {
            INode::open(current, &filename)?
        };
        // if flags.contains(OpenFlags::WRONLY) {
        //     file.lseek(0, 2);
        // }
        let fd = process.fd_table.alloc();
        process.fd_table.set(fd, FileDesc::new(file));
        drop(process);
        debug!("return fd: {}", fd);
        inner.context.x[10] = fd;
        Ok(())
    }
    // 关闭文件
    pub fn sys_close(&self, fd: usize) -> Result<(), RuntimeError> {
        debug!("close fd: {}", fd);
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        process.fd_table.dealloc(fd);
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_readlinkat(&self, dir_fd: usize, path: UserAddr<u8>, 
        buf: UserAddr<u8>, len: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let path = path.read_string();
        debug!("read {} from dir_fd: {:#x} len: {}", path, dir_fd, len);
        let path = if path == "/proc/self/exe" {
            "/lmbench_all".to_string()
        } else {
            path
        };
        let path = path.as_bytes();

        let buf = buf.transfer_vec(len);
        // let inode = INode::get(None, &path)?;
        // let read_len = inode.read_to(buf)?;
        // debug!("read_len: {:#x}", read_len);
        buf[..path.len()].copy_from_slice(path);
        inner.context.x[10] = path.len();
        Ok(())
    }

    pub fn sys_ppoll(&self, fds: UserAddr<PollFD>, nfds: usize, _timeout: UserAddr<TimeSpec>) -> Result<(), RuntimeError> {
        let fds = fds.transfer_vec(nfds);
        let mut inner = self.inner.borrow_mut();
        debug!("wait for fds: {}", fds.len());
        for i in fds {
            debug!("wait fd: {}", i.fd);
        }
        inner.context.x[10] = 1;
        Ok(())
    }

    // 管道符
    pub fn sys_pipe2(&self, req_ptr: UserAddr<u32>) -> Result<(), RuntimeError> {
        let pipe_arr =  req_ptr.transfer_vec(2);
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        // 创建pipe
        let (read_pipe, write_pipe) = new_pipe();
        // 写入数据
        pipe_arr[0] = process.fd_table.push(read_pipe) as u32;
        pipe_arr[1] = process.fd_table.push(write_pipe) as u32;
                
        drop(process);
        // 创建成功
        inner.context.x[10] = 0;
        Ok(())
    }


}

#[repr(C)]
pub struct PollFD {
    pub fd: u32,
    pub envents: u16,
    pub revents: u16
}