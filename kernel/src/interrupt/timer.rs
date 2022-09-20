use crate::sync::mutex::Mutex;
use crate::sbi::set_timer;
use riscv::register::{sie, time};

#[cfg(not(feature = "board_k210"))]
const CLOCK_FREQ: usize = 12500000;

#[cfg(feature = "board_k210")]
// const CLOCK_FREQ: usize = 4030000000 / 62;
const CLOCK_FREQ: usize = 403000000 / 62;

const CHANGE_TASK_TICKS: usize = 10;

// const INTERVAL: usize = CLOCK_FREQ / 100;
// const INTERVAL: usize = CLOCK_FREQ / 25;
const INTERVAL: usize = CLOCK_FREQ / 1;

const MSEC_PER_SEC: usize = 1000;
const NSEC_PER_SEC: usize = 1_000_000_000;

// tms_utime记录的是进程执行用户代码的时间.
// tms_stime记录的是进程执行内核代码的时间.
// tms_cutime记录的是子进程执行用户代码的时间.
// tms_ustime记录的是子进程执行内核代码的时间.
#[allow(dead_code)]
pub struct TMS
{
	pub tms_utime: u64, 
	pub tms_stime: u64,
	pub tms_cutime: u64,
	pub tms_cstime: u64
}

impl TMS {
    // 创建TMS
    pub fn new() -> Self {
        TMS { tms_utime: 0, tms_stime: 0, tms_cutime: 0, tms_cstime: 0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TimeSpec {
	pub tv_sec: usize,       /* 秒 */
    pub tv_nsec: usize       /* 纳秒, 范围在0~999999999 */
}

impl TimeSpec {
    pub fn get_now(&mut self) {
        let ms = get_time_ms();
        self.tv_sec = ms / 1000;
        self.tv_nsec = (ms % 1000) * 1000;
    }

    pub fn now() -> Self {
        // let ms = get_time_ms();
        // Self{
        //     tv_sec: ms / 1000,
        //     tv_nsec: (ms % 1000) * 1000
        // }
        let tick = time::read();
        Self {
            tv_sec: tick / CLOCK_FREQ,
            tv_nsec: (tick % CLOCK_FREQ) * NSEC_PER_SEC / CLOCK_FREQ,
        }
    }
}

// 获取毫秒结构
pub fn get_time_sec() -> usize {
    time::read() / CLOCK_FREQ
}

pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub fn get_time_us() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC / MSEC_PER_SEC)
}

// 下一个任务ticks
pub struct NextTaskTicks(usize);

impl NextTaskTicks {
    // 创建任务TICKS结构
    pub fn new() -> Self {
        NextTaskTicks(CHANGE_TASK_TICKS)
    }

    // 刷新TICKS
    pub fn refresh(&mut self) {
        self.0 = self.0 + CHANGE_TASK_TICKS;
    }

    // 判断是否需要更换任务
    pub fn need_change(&self, ticks: usize) -> bool {
        ticks > self.0
    }
}

lazy_static! {
    pub static ref NEXT_TICKS: Mutex<NextTaskTicks> = Mutex::new(NextTaskTicks::new());
}

// 时间信息
pub static mut TICKS: usize = 0;
pub static mut LAST_TICKS: usize = 0;

/// 时钟中断处理器
pub fn timer_handler() {
    set_next_timeout();
    unsafe {
        TICKS=TICKS+1;
    }
    // 判断是否需要更换任务
    if NEXT_TICKS.force_get().need_change(unsafe { TICKS }) {
        // suspend_and_run_next();
    }
}

// 设置下一次时钟中断触发时间
fn set_next_timeout() {
    // 调用sbi设置定时器
    set_timer(time::read() + INTERVAL);
}

// 初始化定时器
pub fn init() {
    info!("初始化定时器");
    unsafe {
        // 开启时钟中断
        sie::set_stimer();
        // 允许中断产生
        // sstatus::set_sie();
    }
    // 设置下一次中断产生时间
    set_next_timeout();
}

pub fn task_time_refresh() {
    NEXT_TICKS.force_get().refresh();
    set_next_timeout();
}

#[inline]
pub fn get_ticks() -> usize {
    unsafe {TICKS}
}

#[inline]
pub fn set_last_ticks() {
    unsafe {LAST_TICKS = TICKS};
}