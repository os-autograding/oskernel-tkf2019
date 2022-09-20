use crate::memory::addr::UserAddr;
use crate::task::task::Task;
use crate::task::signal::SigSet;
use crate::task::signal::SigAction;
use crate::runtime_err::RuntimeError;

impl Task {
    pub fn sys_sigprocmask(&self, how: u32, set:  UserAddr<SigSet>, oldset: UserAddr<SigSet>,
            _sigsetsize: usize) -> Result<(), RuntimeError> {
        // let pmm = self.get_pmm();
        let mut inner = self.inner.borrow_mut();
        debug!(
            "rt_sigprocmask: how={:#x}, set={:#?}, oldset={:#?}, sigsetsize={}, thread={}",
            how,
            set.bits(),
            oldset.bits(),
            _sigsetsize,
            self.tid
        );
        if oldset.is_valid() {
            oldset.transfer().copy_from(&inner.sig_mask);
        }
        if set.is_valid() {
            let sig = set.transfer();
            match how {
                // block
                0 => inner.sig_mask.block(sig),
                // unblock
                1 => inner.sig_mask.unblock(sig),
                // setmask
                2 => inner.sig_mask.copy_from(sig),
                _ => unimplemented!()
            }
        }
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_sigaction(&self, signum: usize, act: UserAddr<SigAction>, oldact: UserAddr<SigAction>, 
            _sigsetsize: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();

        if oldact.is_valid() {
            oldact.transfer().copy_from(&process.sig_actions[signum]);
        }
        if act.is_valid() {
            let act = act.transfer();
            debug!(
                "rt_sigaction: signal={:?}, act={:?}, oldact={:?}, sigsetsize={}, thread={}",
                signum,
                act,
                oldact.bits(),
                _sigsetsize,
                self.tid
            );
            process.sig_actions[signum].copy_from(act);
        }
        drop(process);
        inner.context.x[10] = 0;
        Ok(())
    }

    pub fn sys_sigreturn(&self) -> Result<(), RuntimeError> {
        debug!("sig return");
        Err(RuntimeError::SigReturn)
    }
}