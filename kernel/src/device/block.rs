use virtio_drivers::VirtIOBlk;
use crate::virtio_impl::HalImpl;

use super::BlockDevice;

// 虚拟IO设备
pub struct VirtIOBlock(pub VirtIOBlk<'static, HalImpl>);

impl BlockDevice for VirtIOBlock {
    fn read_block(&mut self, sector_offset: usize, buf: &mut [u8]) {
        self.0.read_block(sector_offset, buf).expect("读取失败")
    }

    fn write_block(&mut self, sector_offset: usize, buf: &mut [u8]) {
        self.0.read_block(sector_offset, buf).expect("写入失败")
    }

    fn handle_irq(&mut self) {
        todo!()
    }
}
