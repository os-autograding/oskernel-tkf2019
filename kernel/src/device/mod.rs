pub mod block;
pub mod sdcard;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use fatfs::{Dir as OtherDir, File as OtherFile, FileSystem as OtherFileSystem};
use fatfs::LossyOemCpConverter;
use fatfs::NullTimeProvider;
use virtio_drivers::VirtIOBlk;
use virtio_drivers::VirtIOHeader;
use crate::sync::mutex::Mutex;

use crate::runtime_err::RuntimeError;

use self::block::VirtIOBlock;
use self::sdcard::SDCardWrapper;

pub type Dir = OtherDir<DiskCursor, NullTimeProvider, LossyOemCpConverter>;
pub type DiskFile = OtherFile<DiskCursor, NullTimeProvider, LossyOemCpConverter>;
pub type FileSystem = OtherFileSystem<DiskCursor, NullTimeProvider, LossyOemCpConverter>;

#[cfg(not(feature = "board_k210"))]
pub const VIRTIO0: usize = 0x10001000;

// 存储设备控制器 用来存储读取设备
pub static mut BLK_CONTROL: Vec<Box<dyn BlockDevice>> = vec![];

lazy_static! {
    pub static ref GLOBAL_FS: Mutex<Rc<FileSystem>> = {
        let c = DiskCursor {
            sector: 0,
            offset: 0,
            disk_index: 0
        };
        Mutex::new(Rc::new(fatfs::FileSystem::new(c, fatfs::FsOptions::new()).expect("文件系统初始化失败")))
    };
}

/// 定义trait
pub trait BlockDevice {
    // 读取扇区
    fn read_block(&mut self, sector_offset: usize, buf: &mut [u8]);
    // 写入扇区
    fn write_block(&mut self, sector_offset: usize, buf: &mut [u8]);
    // 处理中断
    fn handle_irq(&mut self);
}

pub fn add_virt_io(virtio: usize) {
    // 创建存储设备
    let device = Box::new(VirtIOBlock(
        VirtIOBlk::new(unsafe {&mut *(virtio as *mut VirtIOHeader)}).expect("failed to create blk driver")
    ));
    // 加入设备表
    unsafe {
        BLK_CONTROL.push(device)
    };
}

#[allow(unused)]
pub fn add_sdcard() {
    // 创建SD存储设备
    let block_device = Box::new(SDCardWrapper::new());

    // 加入存储设备表
    unsafe {
        BLK_CONTROL.push(block_device);
    }
}

// 初始化函数
pub fn init() {
    info!("初始化设备");
    #[cfg(not(feature = "board_k210"))]
    {
        // qemu 时添加 储存设备
        add_virt_io(VIRTIO0);
    }
    #[cfg(feature = "board_k210")]
    {
        // 添加 k210 存储设备
        add_sdcard();
    }
}

pub fn root_dir() -> Dir {
    GLOBAL_FS.lock().to_owned().root_dir()
}

/// 硬盘数据读取器
pub struct DiskCursor {
    pub sector: u64,
    pub offset: usize,
    pub disk_index: usize
}

impl DiskCursor {
    fn get_position(&self) -> usize {
        (self.sector * 0x200) as usize + self.offset
    }

    fn set_position(&mut self, position: usize) {
        self.sector = (position / 0x200) as u64;
        self.offset = position % 0x200;
    }

    fn move_cursor(&mut self, amount: usize) {
        self.set_position(self.get_position() + amount)
    }
}

impl fatfs::IoError for RuntimeError {
    fn is_interrupted(&self) -> bool {
        false
    }

    fn new_unexpected_eof_error() -> Self {
        Self::UnexpectedEof
    }

    fn new_write_zero_error() -> Self {
        Self::WriteZero
    }
}

impl fatfs::IoBase for DiskCursor {
    type Error = RuntimeError;
}

impl fatfs::Read for DiskCursor {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, RuntimeError> {
        // 获取硬盘设备写入器（驱动？）
        let block_device = unsafe { &mut BLK_CONTROL[self.disk_index] };

        let mut i = 0;
        let mut data = [0u8; 512];
        while i < buf.len() {
            block_device.read_block(self.sector as usize, &mut data);
            let data = &data[self.offset..];
            if data.len() == 0 { break; }
            let end = (i + data.len()).min(buf.len());
            let len = end - i;
            buf[i..end].copy_from_slice(&data[..len]);
            i += len;
            self.move_cursor(len);
        }
        Ok(i)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), RuntimeError> {
        let n = self.read(buf)?;
        assert!(n == buf.len(), "TODO: Error");
        Ok(())
    }
}

impl fatfs::Write for DiskCursor {
    fn write(&mut self, buf: &[u8]) -> Result<usize, RuntimeError> {
        // 获取硬盘设备写入器（驱动？）
        let block_device = unsafe { &mut BLK_CONTROL[self.disk_index] };

        let mut data = [0u8; 512];

        if self.offset != 0 || buf.len() != 512 {
            block_device.read_block(self.sector as usize, &mut data);
        }

        let (start, end) = if buf.len() == 512 {
            (0, 512)
        } else {
            (self.offset, self.offset + buf.len())
        };
        data[start..end].clone_from_slice(&buf);
        block_device.write_block(self.sector as usize, &mut data);
        self.move_cursor(buf.len());

        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), RuntimeError> {
        self.write(buf)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), RuntimeError> {
        Ok(())
    }
}

impl fatfs::Seek for DiskCursor {
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, RuntimeError> {
        match pos {
            fatfs::SeekFrom::Start(i) => {
                self.set_position(i as usize);
                Ok(i)
            }
            fatfs::SeekFrom::End(_) => {
                todo!("Seek from end")
            }
            fatfs::SeekFrom::Current(i) => {
                let new_pos = (self.get_position() as i64) + i;
                self.set_position(new_pos as usize);
                Ok(new_pos as u64)
            }
        }
    }
}