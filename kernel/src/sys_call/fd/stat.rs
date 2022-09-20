use crate::{task::{task::Task, fd_table::FD_NULL}, memory::addr::UserAddr, fs::{file::{Kstat, FileType}, filetree::INode, StatFS}, runtime_err::RuntimeError};

impl Task {
    pub fn sys_fstat(&self, fd: usize, buf_ptr: UserAddr<Kstat>) -> Result<(), RuntimeError> {
        debug!("sys_fstat: {}", fd);
        let kstat = buf_ptr.transfer();
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();

        // // 判断文件描述符是否存在
        let inode = process.fd_table.get_file(fd)?;
        let inode = inode.get_inode();
        let _inode = inode.0.borrow_mut();
        kstat.st_dev = 1;
        kstat.st_ino = 1;
        kstat.st_mode = 0;
        kstat.st_nlink = 1;
        kstat.st_uid = 0;
        kstat.st_gid = 0;
        kstat.st_rdev = 0;
        kstat.__pad = 0;
        kstat.st_blksize = 512; // 磁盘扇区大小
        // kstat.st_size = inode.size as u64;
        // kstat.st_blocks = ((inode.size - 1 + 512) / 512) as u64;
        // kstat.st_atime_sec = inode.st_atime_sec;
        // kstat.st_atime_nsec = inode.st_atime_nsec;
        // kstat.st_mtime_sec = inode.st_mtime_sec;
        // kstat.st_mtime_nsec = inode.st_mtime_nsec;
        // kstat.st_ctime_sec = inode.st_ctime_sec;
        // kstat.st_ctime_nsec = inode.st_ctime_nsec;

        // debug
        kstat.st_size = 0;
        kstat.st_blocks = 0;
        kstat.st_atime_sec  = 0;
        kstat.st_atime_nsec = 0;
        kstat.st_mtime_sec  = 0;
        kstat.st_mtime_nsec = 0;
        kstat.st_ctime_sec  = 0;
        kstat.st_ctime_nsec = 0;
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }

    // 获取文件信息
    pub fn sys_fstatat(&self, dir_fd: usize, filename: UserAddr<u8>, stat_ptr: UserAddr<Kstat>, _flags: usize) -> Result<(), RuntimeError> {
        let filename = filename.read_string();
        let kstat = stat_ptr.transfer();
        debug!("sys_fstatat: dir_fd {:#x}, filename: {}, filename_len: {}", dir_fd, filename, filename.len());

        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();

        if filename != "/dev/null" {
            // 判断文件描述符是否存在
            let file = if dir_fd == FD_NULL {
                None
            } else {
                let file = process.fd_table.get_file(dir_fd)?;
                Some(file.get_inode())
            };

            let inode = INode::get(file, &filename)?;
            let inode = inode.0.borrow_mut();
            kstat.st_dev = 1;
            kstat.st_ino = 1;
            // kstat_ptr.st_mode = 0;
            if inode.file_type == FileType::Directory {
                kstat.st_mode = 0o40000;
            } else {
                kstat.st_mode = 0;
            }
            // kstat.st_nlink = inode.nlinkes as u32;
            // kstat.st_uid = 0;
            // kstat.st_gid = 0;
            // kstat.st_rdev = 0;
            // kstat.__pad = 0;
            // kstat.st_size = inode.size as u64;
            // kstat.st_blksize = 512;
            // kstat.st_blocks = ((inode.size - 1 + 512) / 512) as u64;
            // kstat.st_atime_sec = inode.st_atime_sec;
            // kstat.st_atime_nsec = inode.st_atime_nsec;
            // kstat.st_mtime_sec = inode.st_mtime_sec;
            // kstat.st_mtime_nsec = inode.st_mtime_nsec;
            // kstat.st_ctime_sec = inode.st_ctime_sec;
            // kstat.st_ctime_nsec = inode.st_ctime_nsec;
            drop(process);
            inner.context.x[10] = 0;
            Ok(())
        } else {
            // kstat.
            kstat.st_mode = 0o20000;
            drop(process);
            inner.context.x[10] = 0;
            Ok(())
        }
    }

    // 获取文件信息
    pub fn sys_getdents(&self, fd: usize, ptr: UserAddr<u8>, len: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();
        debug!("get dents: fd: {} ptr: {:#x} len: {:#x}", fd, ptr.bits(), len);
        let buf = ptr.transfer_vec(len);
        let dir_file = process.fd_table.get_file(fd)?;
        
        let mut pos = 0;
        while let Some((i, inode)) = dir_file.entry_next() {
            let sub_node_name = inode.get_filename();
            debug!("子节点: {}  filetype: {:?} filename len: {}", sub_node_name, inode.get_file_type(), sub_node_name.len());
            let sub_node_name = sub_node_name.as_bytes();
            let node_size = ((19 + sub_node_name.len() as u16 + 1 + 7) / 8) * 8;
            let next = pos + node_size as usize;
            buf[pos..pos+8].copy_from_slice(&(i as u64).to_ne_bytes());
            pos += 8;
            buf[pos..pos+8].copy_from_slice(&(i as u64).to_ne_bytes());
            pos += 8;
            buf[pos..pos+2].copy_from_slice(&node_size.to_ne_bytes());
            pos += 2;
            // buf[pos] = 8;   // 写入type  支持文件夹类型
            buf[pos] = match inode.get_file_type() {
                FileType::File => 8,
                FileType::Directory => 4,
                _ => 0
            };
            pos += 1;
            buf[pos..pos + sub_node_name.len()].copy_from_slice(sub_node_name);
            // pos += node_size as usize;
            pos += sub_node_name.len();
            buf[pos..next].fill(0);   // 写入结束符
            pos = next;

        }

        drop(process);
        debug!("written size: {}", pos);
        inner.context.x[10] = pos;
        // 运行时使用
        // inner.context.x[10] = 0;
        
        Ok(())
    }

    pub fn sys_statfs(&self, _fd: usize, buf_ptr: UserAddr<StatFS>) -> Result<(), RuntimeError> {
        let buf = buf_ptr.transfer();
        
        buf.f_type = 32;
        buf.f_bsize = 512;
        buf.f_blocks = 80;
        buf.f_bfree = 40;
        buf.f_bavail = 0;
        buf.f_files = 32;
        buf.f_ffree = 0;
        buf.f_fsid = 32;
        buf.f_namelen = 20;

        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }    
}