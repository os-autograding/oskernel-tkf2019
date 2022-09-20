// 分区trait
pub trait Partition {
    fn read_sector(&self, sector_offset: usize, buf: &mut [u8]);                // 读取扇区
    fn write_sector(&self, sector_offset: usize, buf: &mut [u8]);               // 写入扇区
    fn mount(&self, prefix: &str);                                              // 获取文件树
}