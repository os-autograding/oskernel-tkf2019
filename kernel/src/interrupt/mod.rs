pub mod timer;

use core::arch::global_asm;
use core::arch::asm;
use riscv::register::scause::Trap;
use riscv::register::scause::Exception;
use riscv::register::scause::Interrupt;
use riscv::register::scause::Scause;

pub use timer::TICKS;


#[repr(C)]
#[derive(Debug, Clone)]
// 上下文
pub struct Context {
    pub x: [usize; 32],     // 32 个通用寄存器
    pub sstatus: usize,
    pub sepc: usize
}

impl Context {
    // 创建上下文信息
    pub fn new() -> Self {
        Context {
            x: [0usize; 32],
            sstatus: 0,
            sepc: 0
        }
    }
    // 从另一个上下文复制
    pub fn clone_from(&mut self, target: &Self) {
        for i in 0..32 {
            self.x[i] = target.x[i];
        }

        self.sstatus = target.sstatus;
        self.sepc = target.sepc;
    }
}

// break中断
fn breakpoint(context: &mut Context) {
    warn!("break中断产生 中断地址 {:#x}", context.sepc);
    context.sepc = context.sepc + 2;
}

// 中断错误
fn fault(context: &mut Context, _scause: Scause, _stval: usize) {
    debug!("中断 {:#x} 地址 {:#x} stval: {:#x}", _scause.bits(), context.sepc, _stval);
    debug!("a0: {:#x}, a1: {:#x}
    a2: {:#x}, a3: {:#x}
    a4: {:#x}, a5: {:#x}, 
    a6: {:#x}, a7: {:#x}", 
    context.x[10], context.x[11],
    context.x[12], context.x[13],
    context.x[14], context.x[15],
    context.x[16], context.x[17],
    );
    panic!("未知中断")
}

// 处理缺页异常
fn handle_page_fault(_context: &mut Context, _stval: usize) {
    warn!("缺页中断触发 缺页地址: {:#x} 触发地址:{:#x} 已同步映射", _stval, _context.sepc);
    panic!("end");
}

// 内核中断回调
#[no_mangle]
fn kernel_callback(context: &mut Context, scause: Scause, stval: usize) -> usize {
    warn!("内核态中断发生: {:#x}  stval {:#x}  sepc: {:#x}", scause.bits(), stval,  context.sepc);
    match scause.cause(){
        // 中断异常
        Trap::Exception(Exception::Breakpoint) => breakpoint(context),
        // 时钟中断 eg: 不再内核处理时间中断 just in user
        Trap::Interrupt(Interrupt::SupervisorTimer) => {},
        // 缺页异常
        Trap::Exception(Exception::StorePageFault) => handle_page_fault(context, stval),
        // 加载页面错误
        Trap::Exception(Exception::LoadPageFault) => {
            panic!("加载权限异常 地址:{:#x}", stval)
        },
        Trap::Exception(Exception::InstructionPageFault) => handle_page_fault(context, stval),
        // 页面未对齐异常
        Trap::Exception(Exception::StoreMisaligned) => {
            info!("页面未对齐");
        }
        // 其他情况，终止当前线程
        _ => fault(context, scause, stval),
    }
    context as *const Context as usize
}


// 包含中断代码
global_asm!(include_str!("interrupt-kernel.asm"));

// 设置中断
pub fn init() {
    extern "C" {
        fn kernel_callback_entry();
    }

    // 输出内核信息
    info!("kernel_callback_entry addr: {:#x}", kernel_callback_entry as usize);

    unsafe {
        asm!("csrw stvec, a0", in("a0") kernel_callback_entry as usize);
    }

    // 初始化定时器
    timer::init();
}

// 调试代码
pub fn test() {
    unsafe {asm!("ebreak")};
}