use alloc::vec::Vec;
use riscv::register::scause;
use riscv::register::scause::Trap;
use riscv::register::scause::Exception;
use riscv::register::scause::Interrupt;
use riscv::register::stval;
use riscv::register::sstatus;
use crate::interrupt::timer;
use crate::sync::mutex::Mutex;
use crate::sys_call::consts::ENOENT;
use crate::task::task_scheduler::kill_task;
use crate::sys_call::consts::EBADF;
use crate::interrupt::timer::set_last_ticks;
use crate::runtime_err::RuntimeError;
use crate::task::signal::SignalUserContext;
use crate::task::task::Task;
use crate::task::task_scheduler::switch_next;

pub mod fd;
pub mod task;
pub mod time;
pub mod mm;
pub mod consts;
pub mod signal;
pub mod net;

// 中断调用列表
pub const SYS_GETCWD:usize  = 17;
pub const SYS_DUP: usize    = 23;
pub const SYS_DUP3: usize   = 24;
pub const SYS_FCNTL: usize  = 25;
pub const SYS_MKDIRAT:usize = 34;
pub const SYS_UNLINKAT:usize= 35;
pub const SYS_UMOUNT2: usize= 39;
pub const SYS_MOUNT: usize  = 40;
pub const SYS_STATFS: usize = 43;
pub const SYS_CHDIR: usize  = 49;
pub const SYS_OPENAT:usize  = 56;
pub const SYS_CLOSE: usize  = 57;
pub const SYS_PIPE2: usize  = 59;
pub const SYS_GETDENTS:usize= 61;
pub const SYS_LSEEK: usize  = 62;
pub const SYS_READ:  usize  = 63;
pub const SYS_WRITE: usize  = 64;
pub const SYS_READV:  usize  = 65;
pub const SYS_WRITEV: usize = 66;
pub const SYS_PREAD: usize  = 67;
pub const SYS_SENDFILE: usize = 71;
pub const SYS_PPOLL: usize = 73;
pub const SYS_READLINKAT: usize = 78;
pub const SYS_FSTATAT: usize= 79;
pub const SYS_FSTAT: usize  = 80;
pub const SYS_UTIMEAT:usize = 88;
pub const SYS_EXIT:  usize  = 93;
pub const SYS_EXIT_GROUP: usize = 94;
pub const SYS_SET_TID_ADDRESS: usize = 96;
pub const SYS_FUTEX: usize  = 98;
pub const SYS_NANOSLEEP: usize = 101;
pub const SYS_GETTIME: usize = 113;
pub const SYS_SCHED_YIELD: usize = 124;
pub const SYS_KILL: usize = 129;
pub const SYS_TKILL: usize = 130;
pub const SYS_TGKILL: usize = 131;
pub const SYS_SIGACTION: usize = 134;
pub const SYS_SIGPROCMASK: usize = 135;
pub const SYS_SIGTIMEDWAIT: usize = 137;
pub const SYS_SIGRETURN: usize = 139;
pub const SYS_TIMES: usize  = 153;
pub const SYS_UNAME: usize  = 160;
pub const SYS_GETRUSAGE: usize = 165;
pub const SYS_GETTIMEOFDAY: usize= 169;
pub const SYS_GETPID:usize  = 172;
pub const SYS_GETPPID:usize = 173;
pub const SYS_GETUID: usize = 174;
// pub const SYS_GETEUID: usize = 175;
pub const SYS_GETGID: usize = 176;
pub const SYS_GETTID: usize = 178;
pub const SYS_SOCKET: usize = 198;
pub const SYS_BIND: usize   = 200;
pub const SYS_LISTEN: usize = 201;
pub const SYS_CONNECT: usize = 203;
pub const SYS_GETSOCKNAME: usize = 204;
pub const SYS_SENDTO: usize = 206;
pub const SYS_RECVFROM: usize = 207;
pub const SYS_SETSOCKOPT: usize = 208;
pub const SYS_BRK:   usize  = 214;
pub const SYS_CLONE: usize  = 220;
pub const SYS_EXECVE:usize  = 221;
pub const SYS_MMAP: usize   = 222;
pub const SYS_MPROTECT:usize= 226;
pub const SYS_MUNMAP:usize  = 215;
pub const SYS_WAIT4: usize  = 260;

// 系统调用错误码
pub const SYS_CALL_ERR: usize = -1 as isize as usize;


// Open标志
bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 6;
        const TRUNC = 1 << 10;
        const O_DIRECTORY = 1 << 21;
    }

    pub struct SignalFlag: usize {
        const SA_NOCLDSTOP = 0x1;
        const SA_NOCLDWAIT = 0x2;
        const SA_SIGINFO   = 0x4;
        const SA_RESTART   = 0x10000000;
        const SA_NODEFER   = 0x40000000;
        const SA_RESETHAND = 0x80000000;
        const SA_RESTORER  = 0x04000000;
    }

    pub struct CloneFlags: usize {
        const CSIGNAL		= 0x000000ff;
        const CLONE_VM	= 0x00000100;
        const CLONE_FS	= 0x00000200;
        const CLONE_FILES	= 0x00000400;
        const CLONE_SIGHAND	= 0x00000800;
        const CLONE_PIDFD	= 0x00001000;
        const CLONE_PTRACE	= 0x00002000;
        const CLONE_VFORK	= 0x00004000;
        const CLONE_PARENT	= 0x00008000;
        const CLONE_THREAD	= 0x00010000;
        const CLONE_NEWNS	= 0x00020000;
        const CLONE_SYSVSEM	= 0x00040000;
        const CLONE_SETTLS	= 0x00080000;
        const CLONE_PARENT_SETTID	= 0x00100000;
        const CLONE_CHILD_CLEARTID	= 0x00200000;
        const CLONE_DETACHED	= 0x00400000;
        const CLONE_UNTRACED	= 0x00800000;
        const CLONE_CHILD_SETTID	= 0x01000000;
        const CLONE_NEWCGROUP	= 0x02000000;
        const CLONE_NEWUTS	= 0x04000000;
        const CLONE_NEWIPC	= 0x08000000;
        const CLONE_NEWUSER	= 0x10000000;
        const CLONE_NEWPID	= 0x20000000;
        const CLONE_NEWNET	= 0x40000000;
        const CLONE_IO	= 0x80000000;
    }
}

// 系统信息结构
pub struct UTSname  {
    sysname: [u8;65],
    nodename: [u8;65],
    release: [u8;65],
    version: [u8;65],
    machine: [u8;65],
    domainname: [u8;65],
}

// 文件Dirent结构
#[repr(C)]
#[allow(unused)]
struct Dirent {
    d_ino: u64,	        // 索引结点号
    d_off: u64,	        // 到下一个dirent的偏移
    d_reclen: u16,	    // 当前dirent的长度
    d_type: u8,	        // 文件类型
    // d_name_start: u8	//文件名 文件名 自行处理？
}

impl Task {
    // 系统调用
    pub fn sys_call(&self, call_type: usize, args: [usize; 7]) -> Result<(), RuntimeError> {
        // 匹配系统调用 a7(x17) 作为调用号
        match call_type {
            // 获取文件路径
            SYS_GETCWD => self.get_cwd(args[0].into(), args[1]),
            // 复制文件描述符
            SYS_DUP => self.sys_dup(args[0]),
            // 复制文件描述符
            SYS_DUP3 => self.sys_dup3(args[0], args[1]),
            // 控制资源
            SYS_FCNTL => self.sys_fcntl(args[0], args[1], args[2]),
            // 创建文件夹
            SYS_MKDIRAT => self.sys_mkdirat(args[0], args[1].into(), args[2]),
            // 取消link
            SYS_UNLINKAT => self.sys_unlinkat(args[0], args[1].into(), args[2]),
            // umount设备
            SYS_UMOUNT2 => Ok(()),
            // mount设备
            SYS_MOUNT => Ok(()),
            // 获取文件系统信息
            SYS_STATFS => self.sys_statfs(args[0], args[1].into()),
            // 改变文件信息
            SYS_CHDIR => self.sys_chdir(args[0].into()),
            // 打开文件地址
            SYS_OPENAT => self.sys_openat(args[0], args[1].into(), args[2], args[3]),
            // 关闭文件描述符
            SYS_CLOSE => self.sys_close(args[0]),
            // 进行PIPE
            SYS_PIPE2 => self.sys_pipe2(args[0].into()),
            // 获取文件节点
            SYS_GETDENTS => self.sys_getdents(args[0], args[1].into(), args[2]),
            // 移动读取位置
            SYS_LSEEK => self.sys_lseek(args[0], args[1], args[2]),
            // 读取文件描述符
            SYS_READ => self.sys_read(args[0], args[1].into(), args[2]),
            // 写入文件数据
            SYS_WRITE => self.sys_write(args[0], args[1].into(), args[2]),
            // 读取数据
            SYS_READV => self.sys_readv(args[0], args[1].into(), args[2]),
            // 写入数据
            SYS_WRITEV => self.sys_writev(args[0], args[1].into(), args[2]),
            // 读取数据
            SYS_PREAD => self.sys_pread(args[0], args[1].into(), args[2], args[3]),
            // 发送文件
            SYS_SENDFILE => self.sys_sendfile(args[0], args[1], args[2], args[3]),
            // 等待ppoll
            SYS_PPOLL => self.sys_ppoll(args[0].into(), args[1], args[2].into()),
            // 读取文件数据
            SYS_READLINKAT => self.sys_readlinkat(args[0], args[1].into(), args[2].into(), args[3]),
            // 获取文件数据信息
            SYS_FSTATAT => self.sys_fstatat(args[0], args[1].into(), args[2].into(), args[3]),
            // 获取文件数据信息
            SYS_FSTAT => self.sys_fstat(args[0], args[1].into()),
            // 改变文件时间
            SYS_UTIMEAT => self.sys_utimeat(args[0], args[1].into(), args[2].into(), args[3]),
            // 退出文件信息
            SYS_EXIT => self.sys_exit(args[0]),
            // 退出组
            SYS_EXIT_GROUP => self.sys_exit_group(args[0]),
            // 设置tid
            SYS_SET_TID_ADDRESS => self.sys_set_tid_address(args[0].into()),
            // 互斥锁
            SYS_FUTEX => self.sys_futex(args[0].into(), args[1] as u32, args[2] as _, args[3], args[4]),
            // 文件休眠
            SYS_NANOSLEEP => self.sys_nanosleep(args[0].into(), args[1].into()),
            // 获取系统时间
            SYS_GETTIME => self.sys_gettime(args[0], args[1].into()),
            // 转移文件权限
            SYS_SCHED_YIELD => self.sys_sched_yield(),
            // 结束进程
            SYS_KILL => self.sys_kill(args[0], args[1]),
            // 结束任务进程
            SYS_TKILL => self.sys_tkill(args[0], args[1]),
            // 结束进程
            SYS_TGKILL => self.sys_tgkill(args[0], args[1], args[2]),
            // 释放sigacrtion
            SYS_SIGACTION => self.sys_sigaction(args[0], args[1].into(),args[2].into(), args[3]),
            // 遮盖信号
            SYS_SIGPROCMASK => self.sys_sigprocmask(args[0] as _, args[1].into(),args[2].into(), args[3] as _),
            //
            // SYS_SIGTIMEDWAIT => {
            //     let mut inner = self.inner.borrow_mut();
            //     inner.context.x[10] = 0;
            //     Ok(())
            // }
            // 信号返回程序
            SYS_SIGRETURN => self.sys_sigreturn(),
            // 获取文件时间
            SYS_TIMES => self.sys_times(args[0]),
            // 获取系统信息
            SYS_UNAME => self.sys_uname(args[0].into()),
            // 获取任务获取信息
            SYS_GETRUSAGE => self.sys_getrusage(args[0], args[1].into()),
            // 获取时间信息
            SYS_GETTIMEOFDAY => self.sys_gettimeofday(args[0]),
            // 获取进程信息
            SYS_GETPID => self.sys_getpid(),
            // 获取进程父进程
            SYS_GETPPID => self.sys_getppid(),
            // 获取uid
            SYS_GETUID => {
                self.update_context(|x| x.x[10] = 1);
                Ok(())
            },
            // 获取gid
            SYS_GETGID => {
                self.update_context(|x| x.x[10] = 1);
                Ok(())
            },
            // 获取tid
            SYS_GETTID => self.sys_gettid(),
            // 申请socket
            SYS_SOCKET => self.sys_socket(args[0], args[1], args[2]),
            // 绑定
            SYS_BIND => self.sys_bind(),
            // 监听socket
            SYS_LISTEN => self.sys_listen(),
            // 连接connect
            SYS_CONNECT => self.sys_connect(),
            // 获取socket名称
            SYS_GETSOCKNAME => self.sys_getsockname(),
            // 发送
            SYS_SENDTO => self.sys_sendto(args[0], args[1].into(), args[2], args[3], args[4].into(), args[5]),
            // 接收数据
            SYS_RECVFROM => self.sys_recvfrom(args[0],args[1].into(), args[2], args[3], args[4].into(), args[5]),
            // 设置socket属性
            SYS_SETSOCKOPT => self.sys_setsockopt(),
            // 申请堆空间
            SYS_BRK => self.sys_brk(args[0]),
            // 复制进程信息
            SYS_CLONE => self.sys_clone(args[0], args[1], args[2].into(), args[3], args[4].into()),
            // 执行文件
            SYS_EXECVE => self.sys_execve(args[0].into(), args[1].into(), args[2].into()),
            // 进行文件映射
            SYS_MMAP => self.sys_mmap(args[0], args[1], args[2], args[3], args[4], args[5]),
            // 页面保护
            SYS_MPROTECT => self.sys_mprotect(args[0], args[1], args[2]),
            // 取消文件映射
            SYS_MUNMAP => self.sys_munmap(args[0], args[1]),
            // 等待进程
            SYS_WAIT4 => self.sys_wait4(args[0], args[1].into(), args[2]),
            _ => {
                warn!("未识别调用号 {}", call_type);
                Ok(())
            }
        }
    }

    pub fn catch(&self) {
        let result = self.interrupt();
        if let Err(err) = result {
            match err {
                RuntimeError::KillCurrentTask => {
                    kill_task(self.pid, self.tid);
                }
                RuntimeError::NoEnoughPage => {
                    panic!("No Enough Page");
                }
                RuntimeError::NoMatchedFileDesc => {
                    let mut inner = self.inner.borrow_mut();
                    warn!("未找到匹配的文件描述符");
                    inner.context.x[10] = SYS_CALL_ERR;
                }
                RuntimeError::FileNotFound => {
                    let mut inner = self.inner.borrow_mut();
                    warn!("文件未找到");
                    inner.context.x[10] = ENOENT;
                }
                RuntimeError::EBADF => {
                    let mut inner = self.inner.borrow_mut();
                    warn!("文件未找到  EBADF");
                    inner.context.x[10] = EBADF;
                }
                // 统一处理任务切换
                RuntimeError::ChangeTask => switch_next(),
                _ => {
                    warn!("异常: {:?}", err);
                }
            }
        }
    }

    pub fn signal(&self, signal: usize) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let mut process = inner.process.borrow_mut();

        process.pmm.change_satp();
        
        let sig_action = process.sig_actions[signal];

        let handler = sig_action.handler;
        // 如果没有处理器
        if handler == 0 {
            return Ok(());
        }
        // 保存上下文
        let mut temp_context = inner.context.clone();
        let pmm = process.pmm.clone();
        // 获取临时页表 对数据进行处理
        let ucontext = process.heap.get_temp(pmm)?.tranfer::<SignalUserContext>();
        // 中断正在处理中
        if ucontext.context.x[0] != 0 {
            return Ok(());
        }
        let restorer = sig_action.restorer;
        let _flags = SignalFlag::from_bits_truncate(sig_action.flags);
        
        drop(process);
        inner.context.sepc = handler;
        inner.context.x[1] = restorer;
        inner.context.x[10] = signal;
        inner.context.x[11] = 0;
        inner.context.x[12] = 0xe0000000;
        ucontext.context.clone_from(&temp_context);
        ucontext.context.x[0] = ucontext.context.sepc;
        drop(inner);

        debug!("handle signal: {}  handler: {:#x}", signal, handler);
        loop {
            self.run();
            if let Err(RuntimeError::SigReturn) = self.interrupt() {
                break;
            }
        }
        // 修改回调地址
        temp_context.sepc = ucontext.context.x[0];

        // 恢复上下文 并 移除临时页
        let mut inner = self.inner.borrow_mut();
        let process = inner.process.borrow_mut();
        process.heap.release_temp();
        drop(process);
        inner.context.clone_from(&temp_context);
        Ok(())
    }

    pub fn interrupt(&self) -> Result<(), RuntimeError> {
        unsafe {
            sstatus::set_fs(sstatus::FS::Dirty);
        }
        let scause = scause::read();
        let stval = stval::read();
        let mut task_inner = self.inner.borrow_mut();
        let context = &mut task_inner.context;
        // warn!("中断发生: {:#x}, 地址: {:#x}", scause.bits(), context.sepc);
        // 更新TICKS
        set_last_ticks();

        // 匹配中断原因
        match scause.cause(){
            // 断点中断
            Trap::Exception(Exception::Breakpoint) => {
                warn!("break中断产生 中断地址 {:#x}", context.sepc);
                context.sepc = context.sepc + 2;
            },
            // 时钟中断
            Trap::Interrupt(Interrupt::SupervisorTimer) => {
                timer::timer_handler();
            },
            // 页处理错误
            Trap::Exception(Exception::StorePageFault) | Trap::Exception(Exception::StoreFault) => {
                error!("缺页中断触发 缺页地址: {:#x} 触发地址:{:#x} 已同步映射", stval, context.sepc);
                drop(context);
                if stval > 0xef00_0000 && stval < 0xf00010000 {
                    error!("处理缺页中断;");
                    let mut process = task_inner.process.borrow_mut();
                    process.stack.alloc_until(stval)?;
                } else {
                    panic!("无法 恢复的缺页中断");
                }
            },
            // 用户请求
            Trap::Exception(Exception::UserEnvCall) => {
                // 将 恢复地址 + 4 跳过调用地址
                // if context.x[17] != 113 && context.x[17] != 173 && context.x[17] != 165 && context.x[17] != 64
                // && context.x[17] != 57 && context.x[17] != 63 {
                //     debug!("中断号: {} 调用地址: {:#x}", context.x[17], context.sepc);
                // }
                debug!("中断号: {} 调用地址: {:#x}", context.x[17], context.sepc);

                // 对sepc + 4
                context.sepc += 4;
                // 复制参数
                let mut args = [0;7];
                args.copy_from_slice(&context.x[10..17]);
                let call_type = context.x[17];
                drop(context);
                drop(task_inner);
                

                self.sys_call(call_type, args)?;
            },
            // 加载页面错误
            Trap::Exception(Exception::LoadPageFault) => {
                panic!("加载权限异常 地址:{:#x} 调用地址: {:#x}", stval, context.sepc)
            },
            // 页面未对齐错误
            Trap::Exception(Exception::StoreMisaligned) => {
                warn!("页面未对齐");
            }
            Trap::Exception(Exception::IllegalInstruction) => {
                warn!("中断 {:#x} 地址 {:#x} stval: {:#x}", scause.bits(), context.sepc, stval);
                // panic!("指令页错误");

            }
            Trap::Exception(Exception::InstructionPageFault) => {
                warn!("中断 {:#x} 地址 {:#x} stval: {:#x}", scause.bits(), context.sepc, stval);
                panic!("指令页错误");
            }
            // 其他情况，终止当前线程
            _ => {
                warn!("未知 中断 {:#x} 地址 {:#x} stval: {:#x}", scause.bits(), context.sepc, stval);
                return Err(RuntimeError::KillCurrentTask);
            },
        }
    
        // 更新TICKS
        set_last_ticks();

        Ok(())
    }
}

lazy_static! {
    pub static ref VFORK_WAIT_LIST: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

pub fn is_vfork_wait(pid: usize) -> bool {
    let vfork_wait_list = VFORK_WAIT_LIST.lock();
    vfork_wait_list.iter().find(|&&x| x == pid).is_some()
}

pub fn add_vfork_wait(pid: usize) {
    let mut vfork_wait_list = VFORK_WAIT_LIST.lock();
    vfork_wait_list.push(pid);
}

pub fn remove_vfork_wait(pid: usize) {
    let mut vfork_wait_list = VFORK_WAIT_LIST.lock();
    vfork_wait_list.drain_filter(|&mut x| x==pid);
}