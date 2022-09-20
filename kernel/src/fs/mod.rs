pub mod file;
mod partition;
pub mod filetree;
pub mod stdio;
pub mod cache;
pub mod specials;
pub mod virt_file;

pub use partition::Partition;

use crate::device::root_dir;

#[repr(C)]
pub struct StatFS{
    pub f_type: u64,        //文件系统的类型
    pub f_bsize: u64,       //经优化后的传输块的大小
    pub f_blocks: u64,      //文件系统数据块总数
    pub f_bfree: u64,       //可用块数
    pub f_bavail: u64,      //普通用户能够获得的块数
    pub f_files: u64,       //文件结点总数
    pub f_ffree: u64,       //可用文件结点数
    pub f_fsid: u64,        //文件系统标识
    pub f_namelen: u64,     //文件名的最大长度
}

// 初始化文件系统
pub fn init() {
    // 不再进行文件系统的初始化？ 等待处理 
    filetree::init("/", root_dir());
    info!("初始化文件系统");
}