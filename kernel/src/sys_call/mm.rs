use crate::fs::file::FileOP;
use crate::memory::mem_map::MemMap;
use crate::memory::mem_map::MapFlags;
use crate::memory::page::alloc;
use crate::memory::page::get_free_page_num;
use crate::memory::page_table::PTEFlags;
use crate::runtime_err::RuntimeError;
use crate::task::task::Task;
use crate::memory::addr::PAGE_SIZE;
use crate::memory::addr::VirtAddr;
use crate::memory::addr::get_buf_from_phys_addr;
use crate::task::fd_table::FD_NULL;
use crate::task::fd_table::FD_RANDOM;

impl Task {
    pub fn sys_brk(&self, top_pos: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        if top_pos == 0 {
            let top = process.heap.get_heap_top();
            drop(process);
            debug!("[sys_brk] brk_addr: {:X}; new_addr: {:X} caller addr: {:X}", top_pos, top, inner.context.sepc);
            inner.context.x[10] = top;
        } else {
            let ret = if top_pos > process.heap.get_heap_top() + PAGE_SIZE {
                process.heap.get_heap_top()
            } else {
                process.heap.set_heap_top(top_pos)?
            };
            debug!("[sys_brk] brk_addr: {:X}; new_addr: {:X} caller addr: {:X}", top_pos, ret, inner.context.sepc);
            drop(process);
            inner.context.x[10] = ret;
        }
        Ok(())
    }

    pub fn sys_mmap(&self, start: usize, len: usize, _prot: usize, 
            flags: usize, fd: usize, offset: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();
        debug!("start: {:#x}, len: {}", start, len);
        let start = if start == 0 {
            let latest_addr = process.mem_set.get_last_addr();
            if latest_addr < 0xd000_0000 {
                0xd000_0000
            } else {
                latest_addr
            }
        } else {
            start
        };
        if len == 0x80000 || len == 524288 {
            debug!("wrap? len: {}", len / PAGE_SIZE);
            let start_page = start / PAGE_SIZE;
            debug!("start: {:#x}", start);
            let end_page = start_page + (len / PAGE_SIZE);
            debug!("free pae: {:#x}  start_page: {:#x} end_page: {:#x}", get_free_page_num(), start_page, end_page);
            let mem_map = MemMap::new(start_page.into(), 1, PTEFlags::UVRWX)?;
            for i in start_page..end_page {
                process.pmm.add_mapping(mem_map.ppn, i.into(), PTEFlags::UVRWX)?;
            }
            process.mem_set.0.push(mem_map);
            drop(process);
            inner.context.x[10] = start;
            return Ok(());
        }
        debug!("mmap start: {:#x}, len: {:#x}, prot: {}, flags: {}, fd: {:#x}, offset: {:#x}", start, len, _prot, flags, fd, offset);
        let flags = MapFlags::from_bits_truncate(flags as u32);
        let mut p_start = process.pmm.get_phys_addr(start.into())?;
        debug!("申请: {:#x}", p_start.0);
        if p_start.0 < 0x8000_0000 {
            let page_num = len / PAGE_SIZE;
            let mem_map = MemMap::new(VirtAddr::from(start).into(), page_num, PTEFlags::UVRWX)?;
            p_start = mem_map.ppn.into();
            process.pmm.add_mapping_by_map(&mem_map)?;

            // let parent = process.parent.clone();
            // if let Some(parent) = parent.map_or(None, |x| x.upgrade()) {
            //     let mut parent = parent.borrow_mut();
            //     parent.pmm.add_mapping_by_map(&mem_map)?;
            //     parent.mem_set.0.push(mem_map.clone());

            //     let parent = parent.parent.clone();
            //     if let Some(parent) = parent.map_or(None, |x| x.upgrade()) {
            //         let mut parent = parent.borrow_mut();
            //         parent.pmm.add_mapping_by_map(&mem_map)?;
            //         parent.mem_set.0.push(mem_map.clone());

            //     }

            // }
            process.mem_set.0.push(mem_map);
        }
        let buf = get_buf_from_phys_addr(p_start, len);

        if flags.contains(MapFlags::MAP_FIXED) {
            warn!("contains: fixed");
        }
        if fd == FD_NULL {
            todo!()
        } else if fd == FD_RANDOM {
            drop(process);
            inner.context.x[10] = start;
            Ok(())
        } else {
            let file = process.fd_table.get_file(fd)?;
            debug!("file size: {:#x}", file.get_size());
            file.copy_to(offset, buf);
            drop(process);
            inner.context.x[10] = start;
            Ok(())
        }
    }

    pub fn sys_mprotect(&self, _addr: usize, _len: usize, _prot: usize) -> Result<(), RuntimeError> {
        debug!("保护页面: {:#x}  len: {:#x}", _addr, _len);
        let mut inner = self.inner.borrow_mut();
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_munmap(&self, start: usize, _len: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();
        process.pmm.remove_mapping(start.into());
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }
}