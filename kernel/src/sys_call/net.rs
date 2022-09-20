
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use core::cell::RefCell;
use crate::fs::file::FileOP;
use crate::fs::file::fcntl_cmd;
use crate::memory::addr::UserAddr;
use crate::runtime_err::RuntimeError;
use crate::task::fd_table::FileDesc;
use crate::task::task::Task;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SaFamily(u32);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocketAddr {
    sa_family: SaFamily,
    sa_data: [u8; 14],
}

pub struct SocketFile(RefCell<VecDeque<u8>>);

impl SocketFile {
    fn new() -> Rc<Self> {
        Rc::new(SocketFile(RefCell::new(VecDeque::new())))
    }
}

impl FileOP for SocketFile {
    fn readable(&self) -> bool {
        todo!()
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read_at(&self, _pos: usize, buf: &mut [u8]) -> usize {
        let mut read_index = 0;
        let mut queue = self.0.borrow_mut();
        loop {
            if read_index >= buf.len() {
                break;
            }

            if let Some(char) = queue.pop_front() {
                buf[read_index] = char;
            } else {
                break;
            }

            read_index = read_index + 1;
        }
        read_index
    }

    fn write_at(&self, _pos: usize, buf: &[u8], count: usize) -> usize {
        let mut write_index = 0;
        let mut queue = self.0.borrow_mut();
        loop {
            if write_index >= buf.len() || write_index >= count {
                break;
            }

            queue.push_back(buf[write_index]);
            write_index = write_index + 1;
        }
        write_index
    }

    fn get_size(&self) -> usize {
        self.0.borrow().len()
    }
}

impl Task {
    pub fn sys_socket(&self, _domain: usize, _ty: usize, _protocol: usize) -> Result<(), RuntimeError> {
        let file = SocketFile::new();
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();

        let fd = process.fd_table.push_sock(FileDesc::new(file));
        drop(process);
        inner.context.x[10] = fd;
        Ok(())
    }

    pub fn sys_bind(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_getsockname(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_setsockopt(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_sendto(&self, _fd: usize, _buf: UserAddr<u8>, _len: usize, _flags: usize,
                            _sa: UserAddr<SocketAddr>, _sa_size: usize) -> Result<(), RuntimeError> {
        // let sa = sa.transfer();
        // let mut inner = self.inner.borrow_mut();
        // let process = inner.process.borrow_mut();
        // let buf = buf.transfer_vec(len);

        // let file = process.fd_table.get(fd)?;

        // let send_size = file.write(buf, buf.len());
        // SOCKET_BUF.lock().socket_buf.insert(sa.clone(), file);
        // drop(process);

        // inner.context.x[10] = send_size;
        Ok(())
    }

    pub fn sys_recvfrom(&self, _fd: usize, _buf: UserAddr<u8>, _len: usize, _flags: usize,
        _sa: UserAddr<SocketAddr>, _addr_len: usize) -> Result<(), RuntimeError> {

        // let sa = sa.transfer();
        // let mut inner = self.inner.borrow_mut();
        // let buf = buf.transfer_vec(len);

        // let file = SOCKET_BUF.lock().socket_buf.get(sa).unwrap().clone();

        // let read_len = file.read(buf);
        // inner.context.x[10] = read_len;
        Ok(())
    }

    pub fn sys_listen(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_connect(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_accept(&self) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_fcntl(&self, fd: usize, cmd: usize, _arg: usize) -> Result<(), RuntimeError> {
        debug!("val: fd {}  cmd {:#x} arg {:#x}", fd, cmd, _arg);
        // let mut inner = self.inner.borrow_mut();
        // let node = self.map.get_mut(&fd).ok_or(SysError::EBADF)?;
        if fd >= 50 {
            // 暂时注释掉 后面使用socket
            // match cmd {
            //     // 复制文件描述符
            //     1 => {
            //         inner.context.x[10] = 1;
            //     }
            //     3 => {
            //         inner.context.x[10] = 0o4000;
            //     },
            //     _n => {
            //         debug!("not imple {}", _n);
            //     },
            // };
        } else {
            match cmd {
                fcntl_cmd::DUPFD_CLOEXEC => {
                    debug!("copy value");
                    self.sys_dup(fd)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
