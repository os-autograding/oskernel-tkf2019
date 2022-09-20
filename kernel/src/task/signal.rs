use crate::interrupt::Context;

pub enum Signal {
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6,
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,
    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGSTKFLT = 16,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,
    SIGTTIN = 21,
    SIGTTOU = 22,
    SIGURG = 23,
    SIGXCPU = 24,
    SIGXFSZ = 25,
    SIGVTALRM = 26,
    SIGPROF = 27,
    SIGWINCH = 28,
    SIGIO = 29,
    SIGPWR = 30,
    SIGSYS = 31,
    // real time signals
    SIGRT32 = 32,
    SIGRT33 = 33,
    SIGRT34 = 34,
    SIGRT35 = 35,
    SIGRT36 = 36,
    SIGRT37 = 37,
    SIGRT38 = 38,
    SIGRT39 = 39,
    SIGRT40 = 40,
    SIGRT41 = 41,
    SIGRT42 = 42,
    SIGRT43 = 43,
    SIGRT44 = 44,
    SIGRT45 = 45,
    SIGRT46 = 46,
    SIGRT47 = 47,
    SIGRT48 = 48,
    SIGRT49 = 49,
    SIGRT50 = 50,
    SIGRT51 = 51,
    SIGRT52 = 52,
    SIGRT53 = 53,
    SIGRT54 = 54,
    SIGRT55 = 55,
    SIGRT56 = 56,
    SIGRT57 = 57,
    SIGRT58 = 58,
    SIGRT59 = 59,
    SIGRT60 = 60,
    SIGRT61 = 61,
    SIGRT62 = 62,
    SIGRT63 = 63,
    SIGRT64 = 64,
}

#[derive(Clone, Copy, Debug)]
pub struct SigSet(u64);

impl SigSet {
    pub fn block(&mut self, set: &SigSet) {
        self.0 |= set.0;
    }

    pub fn unblock(&mut self, set: &SigSet) {
        self.0 ^= self.0 & set.0;
    }

    pub fn copy_from(&mut self, target: &Self) {
        self.0 = target.0;
    }

    pub fn new(val: u64) -> Self {
        Self(val)
    }
}

impl Default for SigSet {
    fn default() -> Self {
        Self(0)
    }
}

impl From<u64> for SigSet {
    fn from(value: u64) -> Self {
        Self (value)
    }
}

impl Into<u64> for SigSet {
    fn into(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SigAction {
    pub handler: usize,
    pub flags: usize,
    pub restorer: usize,
    pub mask: SigSet,
}

impl SigAction {
    pub fn new() -> Self {
        Self {
            handler: 0,
            flags: 0,
            restorer: 0,
            mask: Default::default()
        }
    }

    pub fn copy_from(&mut self, target: &Self) {
        self.handler = target.handler;
        self.flags = target.flags;
        self.restorer = target.restorer;
        self.mask.copy_from(&target.mask);
    }

    pub fn empty() -> Self {
        Self {
            handler: 0,
            flags: 0, 
            restorer: 0,
            mask: SigSet(0)
        }
    }
}

bitflags! {
    pub struct SignalStackFlags : u32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 0x80000000;
    }
}

#[repr(C)]
#[derive(Copy, Clone,  Eq, PartialEq)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: SignalStackFlags,
    pub size: usize,
}


#[repr(C)]
#[derive(Clone)]
pub struct SignalUserContext {
    pub flags: usize,           // 0
    pub link: usize,            // 1
    pub stack: SignalStack,     // 2
    pub sig_mask: SigSet,       // 5
    pub _pad: [u64; 16], // very strange, maybe a bug of musl libc
    pub context: Context,       // pc offset = 22 - 6=16
}