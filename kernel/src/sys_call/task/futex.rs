use crate::{memory::addr::UserAddr, task::task::Task, runtime_err::RuntimeError};

bitflags! {
    struct FutexFlags: u32 {
        const WAIT      = 0;
        const WAKE      = 1;
        const REQUEUE   = 3;
        const FUTEX_WAKE_OP = 5;
        const LOCK_PI   = 6;
        const UNLOCK_PI = 7;
        const PRIVATE   = 0x80;
    }
}

impl Task {
    // wait for futex
    pub fn sys_futex(&self, uaddr: UserAddr<i32>, op: u32, value: i32, _value2: usize, _value3: usize) -> Result<(), RuntimeError> {
        debug!("sys_futex uaddr: {:#x} op: {:#x} value: {:#x}", uaddr.bits(), op, value);
        // let uaddr_ref = uaddr.transfer();
        // let op = FutexFlags::from_bits_truncate(op);
        // let mut inner = self.inner.borrow_mut();
        // let process = inner.process.borrow_mut();

        // let op = op - FutexFlags::PRIVATE;
        // debug!(
        //     "Futex uaddr: {:#x}, op: {:?}, val: {:#x}, val2(timeout_addr): {:x}",
        //     uaddr.bits(), op, value, value2,
        // );
        // match op {
        //     FutexFlags::WAIT => {
        //         if *uaddr_ref == value {
        //             drop(process);
        //             debug!("等待进程");
        //             inner.context.x[10] = 0;
        //             inner.status = TaskStatus::WAITING;
        //             drop(inner);
        //             // futex_wait(uaddr.bits());
        //             switch_next();
        //         } else {
        //             // *uaddr_value -= 1;
        //             drop(process);
        //             inner.context.x[10] = 0;
        //         }
        //     },
        //     FutexFlags::WAKE => {
        //         // *uaddr_value = -1;
        //         drop(process);
        //         debug!("debug for ");
        //         // 值为唤醒的线程数
        //         // let count = futex_wake(uaddr.bits(), value as usize);
        //         // inner.context.x[10] = count;
        //         // debug!("wake count : {}", count);
        //         drop(inner);
        //         switch_next();
        //     }
        //     FutexFlags::REQUEUE => {
        //         drop(process);
        //         inner.context.x[10] = 0;

        //     }
        //     _ => todo!(),
        // }
        // if op.contains(FutexFlags::WAKE) {
        //     // *uaddr_value = 0;
        //     // futex_requeue(uaddr.bits(), value as u32, value2, value3 as u32);
        // }
        Ok(())
    }    
}