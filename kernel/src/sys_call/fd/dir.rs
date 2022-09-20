use crate::{task::{task::Task, fd_table::FD_NULL}, memory::addr::UserAddr, runtime_err::RuntimeError, fs::filetree::INode};

impl Task {
    
    // 获取当前路径
    pub fn get_cwd(&self, buf: UserAddr<u8>, size: usize) -> Result<(), RuntimeError> {
        debug!("get_cwd size: {}", size);
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();

        // 获取参数
        let buf = buf.transfer_vec(size);
        // 获取路径
        let pwd = process.workspace.clone();
        let pwd_buf = pwd.get_pwd();
        // let pwd_buf = process.workspace.as_bytes();
        // 将路径复制到缓冲区
        buf[..pwd_buf.len()].copy_from_slice(pwd_buf.as_bytes());
        drop(process);
        inner.context.x[10] = buf.as_ptr() as usize;
        Ok(())
    }

    // 更改工作目录
    pub fn sys_chdir(&self, filename: UserAddr<u8>) -> Result<(), RuntimeError> {
        let filename = filename.read_string();
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();

        // process.workspace = process.workspace.clone() + "/" + &filename;
        process.workspace = INode::get(Some(process.workspace.clone()), &filename)?;

        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }
    
    // 创建文件
    pub fn sys_mkdirat(&self, dir_fd: usize, filename: UserAddr<u8>, flags: usize) -> Result<(), RuntimeError> {
        let filename = filename.read_string();
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();
        debug!("dir_fd: {:#x}, filename: {}", dir_fd, filename);

        // 判断文件描述符是否存在
        let current = if dir_fd == FD_NULL {
            // 在用户根据目录创建
            None
        } else {
            // 判度是否存在节点
            let file = process.fd_table.get_file(dir_fd)?;
            Some(file.get_inode())
        };
        if filename != "/" {
            INode::mkdir(current, &filename, flags as u16)?;
        }
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }
    // 取消链接文件
    pub fn sys_unlinkat(&self, fd: usize, filename: UserAddr<u8>, _flags: usize) -> Result<(), RuntimeError> {
        let filename = filename.read_string();
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();

        // 判断文件描述符是否存在
        let current = if fd == FD_NULL {
            None
        } else {
            let file = process.fd_table.get_file(fd)?;
            Some(file.get_inode())
        };
        let cnode = INode::get(current, &filename)?;
        cnode.del_self();
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }
    

}