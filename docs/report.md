# 操作系统设计报告

## 参考信息

[os-competition-info/ref-info.md at main · oscomp/os-competition-info · GitHub](https://github.com/oscomp/os-competition-info/blob/main/ref-info.md)

[Exceptions (alexbd.cn)](http://note.alexbd.cn/#/riscv/exceptions)

## 比赛准备

### 设备信息

RISC-V芯片引导位置为`0x80000000`，由于可以使用`rustsbi`，因此在`0x80200000`处加入操作系统内核即可，无需再次编写`bootloader`.

### 添加nightly工具链

在编写操作系统过程中需要用到某些`nightly`功能，因此添加`nightly`工具链。

```sh
rustup install nightly
rustup default nightly
```

### 添加Rust build工具

build工具中包含`rust-objdump`和`rust-objcopy`

```sh
cargo install cargo-binutils
rustup component add llvm-tools-preview
```

### 添加target elf

```sh
rustup target add riscv64imac-unknown-none-elf
```

## 比赛仓库目录和文件描述

```rust
.
├── Cargo.toml                // Cargo文件
├── README.md                // README文件
├── bootloader                // rustsbi引导目录
│   ├── rustsbi-k210.bin    // rustsbi k210文件
│   └── rustsbi-qemu.bin    // rust qemu文件
├── docs                    // 文档目录 部署在pages
│   ├── README.md            // 笔记文档
│   ├── index.html            // 笔记网页
│   └── report.md            // 报告文件
├── fs.img                    // 测试文件系统
├── makefile                // makefile文件
├── os.bin                    // 生成的操作系统镜像
└── src                        // 源代码目录
    ├── console.rs            // 字符输出
    ├── device                // 设备控制模块
    │   ├── block.rs        // VIRTIO Block驱动
    │   ├── mod.rs            // device mod文件
    │   └── sdcard.rs        // SDCARD驱动文件    (来自rCore 进行略微修改)
    ├── entry.asm            // 操作系统入口
    ├── fs                    // 文件系统驱动
    │   ├── fat32            // fat32驱动
    │   │   ├── fat32bpb.rs        // fat32bpb
    │   │   ├── file_trait.rs    // file_trait
    │   │   ├── long_file.rs    // fat32长文件名
    │   │   ├── mod.rs            // fat32驱动 mod文件
    │   │   └── short_file.rs    // fat32短文件名
    │   ├── file.rs                // 文件系统文件
    │   ├── filetree.rs            // 文件树
    │   ├── mod.rs                // 文件系统 mod文件
    │   └── partition.rs        // 分区
    ├── interrupt                    // 中断
    │   ├── interrupt-kernel.asm    // 内核中断入口
    │   ├── interrupt-user.asm        // 应用程序中断入口
    │   ├── mod.rs                    // 中断Mod文件 含有中断处理函数
    │   ├── sys_call.rs                // 系统调用函数
    │   └── timer.rs                // 定时器
    ├── linker-k210.ld                // k210 linker文件
    ├── linker-qemu.ld                // qemu linker文件
    ├── main.rs                        // 操作系统主函数
    ├── memory                        // 内存描述函数
    │   ├── addr.rs                    // 虚拟地址和物理地址描述文件
    │   ├── heap.rs                    // 操作系统堆结构 和 Global_Allocator
    │   ├── mod.rs                    // 内存mod文件
    │   ├── page.rs                    // 内存页 管理器 分配器
    │   └── page_table.rs            // 内存页映射管理器
    ├── panic.rs                    // panic 文件
    ├── sbi.rs                        // sbi调用函数
    ├── sync                        // sync相关函数
    │   ├── mod.rs                    // sync mod文件
    │   └── mutex.rs                // Mutex 定义
    ├── task                        // 任务管理函数
    │   ├── change_task.asm            // 更换任务 汇编代码
    │   ├── mod.rs                    // task mod文件
    │   ├── pipe.rs                    // 任务 pipe文件 包含PipeBuf
    │   └── task_queue.rs            // 任务队列文件
    └── virtio_impl.rs                // virtio_impl申请文件
```

## 分工与协作

杨金博为主要负责人，负责代码编写，包括但不限于操作系统入口、中断、多任务、内存管理 、系统调用的实现。并提供创作思路，分配团队任务，提高团队凝聚力。

王佳慧主要负责操作系统的数据收集，根据系统和用户需求收集相关数据，为系统运行提供一定的数据支撑，是操作系统不可或缺的一部分。

李莉负责操作系统的文档的编写，将整个操作系统的实现过程和方法以文档的形式呈现，让审核人员能快速了解我们的系统。

作为一个团队，由主要负责人安排其余队员的工作并与团队成员及时沟通，确保出现问题能够分级解决。遇到困难我们会分工查阅资料，遇到不懂的数据和信息我们会相互讨论请教，这样一种由上到下、自下而上的分层合作关系让整个操作系统的实现更加顺利。

## 需求分析及基础知识

### 系统调用

```rust
// 系统调用列表
pub const SYS_GETCWD:usize  = 17;
pub const SYS_DUP: usize    = 23;
pub const SYS_DUP3: usize   = 24;
pub const SYS_MKDIRAT:usize = 34;
pub const SYS_UNLINKAT:usize= 35;
pub const SYS_UMOUNT2: usize= 39;
pub const SYS_MOUNT: usize  = 40;
pub const SYS_CHDIR: usize  = 49;
pub const SYS_OPENAT:usize  = 56;
pub const SYS_CLOSE: usize  = 57;
pub const SYS_PIPE2: usize  = 59;
pub const SYS_GETDENTS:usize= 61;
pub const SYS_READ:  usize  = 63;
pub const SYS_WRITE: usize  = 64;
pub const SYS_FSTAT: usize  = 80;
pub const SYS_EXIT:  usize  = 93;
pub const SYS_NANOSLEEP: usize = 101;
pub const SYS_SCHED_YIELD: usize = 124;
pub const SYS_TIMES: usize  = 153;
pub const SYS_UNAME: usize  = 160;
pub const SYS_GETTIMEOFDAY: usize= 169;
pub const SYS_GETPID:usize  = 172;
pub const SYS_GETPPID:usize = 173;
pub const SYS_BRK:   usize  = 214;
pub const SYS_CLONE: usize  = 220;
pub const SYS_EXECVE:usize  = 221;
pub const SYS_MMAP: usize   = 222;
pub const SYS_MUNMAP:usize  = 215;
pub const SYS_WAIT4: usize  = 260;
```

### Rust使用自定义的内存管理分配器(heap)

rust在如果使用no_std即使用core库且需要使用Ref Vec等功能需要自己实现#[global_allocator], 然后才能进行内存的分配

### 字符设置

CR：Carriage Return，对应ASCII中转义字符\r，表示回车

LF：Linefeed，对应ASCII中转义字符\n，表示换行

CRLF：Carriage Return & Linefeed，\r\n，表示回车并换行

### UTF-8字符转换规则

> Unicode 与 UTF-8 编码有一个归纳的转换规则 ：
> Unicode Code    UTF-8 Code
> 0000～007F     0xxxxxxx
> 0080～07FF     110xxxxx 10xxxxxx
> 0800～FFFF     1110xxxx 10xxxxxx 10xxxxxx
> 10000～10FFFF   11110xxx 10xxxxxx 10xxxxxx 10xxxxxx

获取uf8字符后转unicode

转换代码:

```rust
if c as u8 >= 0b11000000 {
    // 获取到utf8字符 转unicode
    console_putchar(c as u8);
    let mut char_u32:u32 = c as u32;
    let times = if c as u8 <= 0b11100000 {
        char_u32 = char_u32 & 0x1f;
        1
    } else if c as u8 <= 0b11110000 {
        char_u32 = char_u32 & 0x0f;
        2
    } else {
        char_u32 = char_u32 & 0x07;
        3
    };


    for _ in 0..times {
        let c = read();
        console_putchar(c as u8);
        char_u32 = char_u32 << 6;
        char_u32 = char_u32 | ((c as u32) & 0x3f
    }

    str.push(char::from_u32(char_u32).unwrap());
    continue;
}
```

### 中断设置

rust中断设置，首先需要设置`stvec`，`stvec`设置中断入口的地址。

设置时钟中断需要置`sie`寄存器的`stie`位开启定时器，并置`sstatus`的`sie`位开启中断。

寄存器详细说明链接: [10. 自制操作系统: risc-v Supervisor寄存器sstatus/stvec/sip/sie_dumpcore的博客-CSDN博客](https://blog.csdn.net/dai_xiangjun/article/details/123967946)

### VIRTIO Proctol

**Virtual I/O protocol**

[参考链接](https://web.eecs.utk.edu/~smarz1/courses/cosc361/notes/virtio/)

[IO Device文档](https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.html)

### MMIO

```rust
#[repr(usize)]
pub enum MmioOffsets {
  MagicValue = 0x000,
  Version = 0x004,
  DeviceId = 0x008,
  VendorId = 0x00c,
  HostFeatures = 0x010,
  HostFeaturesSel = 0x014,
  GuestFeatures = 0x020,
  GuestFeaturesSel = 0x024,
  GuestPageSize = 0x028,
  QueueSel = 0x030,
  QueueNumMax = 0x034,
  QueueNum = 0x038,
  QueueAlign = 0x03c,
  QueuePfn = 0x040,
  QueueNotify = 0x050,
  InterruptStatus = 0x060,
  InterruptAck = 0x064,
  Status = 0x070,
  Config = 0x100,
}
```

rust 读取设备树      

操作系统在启动后需要了解计算机系统中所有接入的设备，这就要有一个读取全部已接入设备信息的能力，而设备信息放在哪里，又是谁帮我们来做的呢？在 RISC-V 中，这个一般是由 bootloader，即 OpenSBI or RustSBI 固件完成的。它来完成对于包括物理内存在内的各外设的探测，将探测结果以 **设备树二进制对象（DTB，Device Tree Blob）** 的格式保存在物理内存中的某个地方。然后bootloader会启动操作系统，即把放置DTB的物理地址将放在 `a1` 寄存器中，而将会把 HART ID （**HART，Hardware Thread，硬件线程，可以理解为执行的 CPU 核**）放在 `a0` 寄存器上，然后跳转到操作系统的入口地址处继续执行。例如，我们可以查看 `virtio_drivers` crate中的在裸机环境下使用驱动程序的例子。我们只需要给 rust_main 函数增加两个参数（即 `a0` 和 `a1` 寄存器中的值 ）即可。

## 系统框架和模块设计

### 1.文件系统

#### FAT32文件系统

[详解FAT32文件系统 - CharyGao - 博客园](https://www.cnblogs.com/Chary/p/12981056.html)

```rust
struct FAT32 {
    device_id: usize,
    fat32bpb: FAT32BPB
}

#[repr(packed)]
pub struct FAT32BPB {
    jmpcode: [u8; 3],       // 跳转代码
    oem: [u8; 8],           // oem 信息
    bytes_per_sector: u16,  // 每扇区字节数
    sectors_per_cluster: u8,// 每簇扇区数
    reserved_sector: u16,   // 保留扇区数 第一个FAT之前的扇区数 包含引导扇区
    fat_number: u8,         // fat表数量
    root_entries: u16,      // 根目录项数 FAT32必须为0
    small_sector: u16,      // 小扇区区数 FAT32必须为0
    media_descriptor: u8,   // 媒体描述符 0xF8标识硬盘 0xF0表示3.5寸软盘
    sectors_per_fat: u16,   // 每FAT扇区数
    sectors_per_track: u16, // 每道扇区数
    number_of_head: u16,    // 磁头数
    hidden_sector: u32,     // 隐藏扇区数
    large_sector: u32,      // 总扇区数
}
```

```rust
// FAT32长文件目录项
#[allow(dead_code)]
#[repr(packed)]
pub struct FAT32longFileItem {
    attr: FAT32FileItemAttr,        // 属性
    filename: [u16; 5],             // 长目录文件名unicode码
    sign: u8,                       // 长文件名目录项标志, 取值0FH
    system_reserved: u8,            // 系统保留
    verification: u8,               // 校验值
    filename1: [u16; 6],            // 长文件名unicode码
    start: u16,                     // 文件起始簇号
    filename2: [u16; 2]              // 长文件名unicode码
}
```

```rust
// FAT32短文件目录项
#[allow(dead_code)]
#[repr(packed)]
pub struct FAT32shortFileItem {
    filename: [u8; 8],          // 文件名
    ext: [u8; 3],               // 扩展名
    attr: FAT32FileItemAttr,    // 属性
    system_reserved: u8,        // 系统保留
    create_time_10ms: u8,       // 创建时间的10毫秒位
    create_time: u16,           // 创建时间
    create_date: u16,           // 创建日期
    last_access_date: u16,      // 最后访问日期
    start_high: u16,            // 起始簇号的高16位
    last_modify_time: u16,      // 最近修改时间
    last_modify_date: u16,      // 最近修改日期
    start_low: u16,             // 起始簇号的低16位
    len: u32                    // 文件长度
}
```

```rust
// 文件项操作接口
pub trait FilesystemItemOperator {
    fn filename(&self) -> String;            // 获取文件名
    fn file_size(&self) -> usize;            // 获取文件大小
    fn start_cluster(&self) -> usize;        // 开始簇
    fn get_attr(&self) -> FAT32FileItemAttr;     // 文件属性
}
```

#### 内核文件树

`ByteOS`采用系统文件树，将文件读取后存储到文件树，以文件树节点作为文件进行操作，此种方式便于文件的添加、修改以及设备的挂载和卸载。同样任务工作目录也是以文件树节点作为指向，文件树以`FileTreeNode`包裹，内部使用`Rc<RefCell<FileTreeNodeRaw>>`智能指针。在其他节点进行修改时，文件树节点也可同步进行修改。

`ByteOS`采用`FAT32`作为文件系统。

```rust
// 文件树原始树
pub struct FileTreeNodeRaw {
    pub filename: String,               // 文件名
    pub file_type: FileType,            // 文件数类型
    pub parent: Option<FileTreeNode>,   // 父节点
    pub children: Vec<FileTreeNode>,    // 子节点
    pub cluster: usize,                 // 开始簇
    pub size: usize,                    // 文件大小
    pub nlinkes: u64,                   // 链接数量
    pub st_atime_sec: u64,              // 最后访问秒
    pub st_atime_nsec: u64,             // 最后访问微秒
    pub st_mtime_sec: u64,              // 最后修改秒
    pub st_mtime_nsec: u64,             // 最后修改微秒
    pub st_ctime_sec: u64,              // 最后创建秒
    pub st_ctime_nsec: u64,             // 最后创建微秒
}
#[derive(Clone)]
// 文件树节点
pub struct FileTreeNode(pub Rc<RefCell<FileTreeNodeRaw>>);
// 文件树
pub struct FileTree(FileTreeNode);
```

### 2.任务调度

#### 任务控制器和任务管理器

```rust
pub struct TaskController {
    pub pid: usize,                                     // 进程id
    pub ppid: usize,                                    // 父进程id
    pub entry_point: VirtAddr,                          // 入口地址
    pub pmm: PageMappingManager,                        // 页表映射控制器
    pub status: TaskStatus,                             // 任务状态
    pub stack: VirtAddr,                                // 栈地址
    pub heap: UserHeap,                                 // 堆地址
    pub context: Context,                               // 寄存器上下文
    pub home_dir: FileTreeNode,                         // 家地址
    pub fd_table: Vec<Option<Arc<Mutex<FileDesc>>>>,    // 任务描述符地址
    pub tms: TMS                                        // 时间地址
}

// 任务控制器管理器
pub struct TaskControllerManager {
    current: Option<Arc<Mutex<TaskController>>>,        // 当前任务
    ready_queue: VecDeque<Arc<Mutex<TaskController>>>,  // 准备队列
    wait_queue: Vec<WaitQueueItem>,                     // 等待队列
    killed_queue: Vec<Arc<Mutex<TaskController>>>,      // 僵尸进程队列
    is_run: bool                                        // 任务运行标志
}
```

#### 进程id生成器

```rust
// PID生成器
pub struct PidGenerater(usize);

impl PidGenerater {
    // 创建进程id生成器
    pub fn new() -> Self {
        PidGenerater(1000)
    }
    // 切换到下一个pid
    pub fn next(&mut self) -> usize {
        let n = self.0;
        self.0 = n + 1;
        n
    }
}
```

#### 文件描述符

```rust
// 文件描述符类型
pub enum FileDescEnum {
    File(FileTreeNode),
    Pipe(PipeBuf),
    Device(String)
}

// 文件描述符
pub struct FileDesc {
    pub target: FileDescEnum,
    pub readable: bool,
    pub writable: bool
}

impl FileDesc {
    // 创建文件描述符
    pub fn new(target: FileDescEnum) -> Self {
        FileDesc {
            target,
            readable: true,
            writable: true
        }
    }

    // 创建pipe
    pub fn new_pipe() -> (Self, Self) {
        ...
    }
}
```

#### 任务切换

```mermaid
graph LR;
    中断开始-->任务调度;
    任务调度-->中断处理;
    中断处理--无需切换任务-->中断结束;
    中断结束-->获取当前任务;
    获取当前任务-->恢复当前任务环境;

    中断处理-->切换任务当前任务;
    切换任务当前任务-->中断结束;
```

#### 等待任务

```mermaid
graph LR;
    中断开始-->从僵尸进程列表中获取需要等待的任务;
    从僵尸进程列表中获取需要等待的任务--存在任务-->移出僵尸进程;
    移出僵尸进程-->写入系统参数;
    写入系统参数-->获取当前任务;

    从僵尸进程列表中获取需要等待的任务--不存在任务-->将当前任务加入等待列表;
    将当前任务加入等待列表-->切换任务;

    切换任务-->获取当前任务;
    获取当前任务-->恢复当前任务环境;
```

### 3.内存管理

#### 页式内存管理

```rust
pub struct MemoryPageAllocator {
    pub start: usize,
    pub end: usize,
    pub pages: Vec<bool>
}

// 添加内存页分配器方法
impl MemoryPageAllocator {
    // 创建内存分配器结构
    fn new() -> Self {
        MemoryPageAllocator {
            start: 0,
            end: 0,
            pages: vec![]
        }
    }

    // 初始化内存分配器
    fn init(&mut self, start: usize, end: usize) {
        ...
    }

    // 申请内存
    pub fn alloc(&mut self) -> Option<PhysPageNum> {
        ...
    }

    // 取消分配页
    pub fn dealloc(&mut self, page: PhysPageNum) {
        ...
    }

    // 申请多个页
    pub fn alloc_more(&mut self, pages: usize) ->Option<PhysPageNum> {
        ...
    }

    // 释放多个页
    pub fn dealloc_more(&mut self, page: PhysPageNum, pages: usize) {
        ...
    }
}
lazy_static! {
    pub static ref PAGE_ALLOCATOR: Mutex<MemoryPageAllocator> = Mutex::new(MemoryPageAllocator::new());
}
```

#### 内存映射管理器

```rust
#[derive(Clone)]
pub struct PageMappingManager {
    pub paging_mode: PagingMode,
    pub pte: PageMapping
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageMapping(usize);

impl PageMapping {
    pub fn new(addr: PhysAddr) -> PageMapping {
        PageMapping(addr.0)
    }

    // 初始化页表
    pub fn alloc_pte(&self, level: usize) -> Option<PhysPageNum> {
        ...
    }

    // 添加mapping
    pub fn add_mapping(&mut self, phy_addr: PhysAddr, virt_addr: VirtAddr, flags:PTEFlags) {
        ...
    }

    // 删除mapping
    pub fn remove_mapping(&mut self, virt_addr: VirtAddr) {
           ...
    }

    // 获取物理地址
    pub fn get_phys_addr(&self, virt_addr: VirtAddr) -> Option<PhysAddr> {
        ...
    }
}

lazy_static! {
    pub static ref KERNEL_PAGE_MAPPING: Mutex<PageMappingManager> = Mutex::new(PageMappingManager::new());
}
```

内存默认情况下进行大页映射，标识符为 `PTEFlags:VRWX `, 因此在任意`satp`中都可执行内核代码。

因为内存映射`PageMapping`内部仅有一个`usize`。因此可以从系统的`satp`寄存器中读取信息，然后转换为`PageMapping`进行读取。

```rust
let mut pmm = PageMapping::from(PhysPageNum(satp::read().bits()).to_addr());
```

## 遇到的主要问题和解决方法

### 1. SYS_GETDENTS 缓冲区溢出异常

在系统调用`SYS_GETDENTS`中对于目录文件进行修改的时候，因为文件内容过多导致缓冲区溢出，在测试案例输出的时候会导致本来输出数字的结果变为输出字母。**在系统调用文件中进行读取字节数限制，修复成功。**

```rust
for i in 0..sub_nodes.len() {
    ...
    // 保证缓冲区不会溢出
    if buf_ptr - start_ptr >= len {
        break;
    }
}
```

### 2. RustSBI多核启动导致数据异常

`rustsbi`在`qemu`中以`Debug`模式启动时只会启动一个核心，但是已`Release`启动时会启动多个核心，在操作系统管理和调试时存在一定问题。**因此目前仅使用一个核心，在操作系统主函数中使用cfg设置其他核心终止，保证仅有一个核心工作。**

```rust
#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, device_tree_paddr: usize) -> ! {
    // // 保证仅有一个核心工作
    #[cfg(not(debug_assertions))]
    if hartid != 0 {
        sbi::hart_suspend(0x00000000, support_hart_resume as usize, 0);
    }
}
```

### 3. 操作系统内核在评测机运行时无法编译

`ByteOS`在早期开发时使用的时`riscv64gc-unknown-none-elf`，但是评测及使用的是`riscv64imac-unknown-none-elf`，因此在编译过程中添加target

```sh
# 编译的目标平台
[build]
# target = "riscv64gc-unknown-none-elf"
target = "riscv64imac-unknown-none-elf"
```

同时在编译时`rustflags`无法使用，因此直接将`rustflags`写入`makefile`中

```makefile
k210: 
    @cp src/linker-k210.ld src/linker.ld
    @RUSTFLAGS="-Clink-arg=-Tsrc/linker.ld" cargo build $(MODE_FLAG) --features "board_k210" --offline
    @rm src/linker.ld
    $(OBJCOPY) $(KERNEL_FILE) --strip-all -O binary $(BIN_FILE)
    @cp $(BOOTLOADER_K210) $(BOOTLOADER_K210).copy
    @dd if=$(BIN_FILE) of=$(BOOTLOADER_K210).copy bs=131072 seek=1
    @mv $(BOOTLOADER_K210).copy $(BIN_FILE)
```

## 辅助函数

### 1. 打印内存

```rust
// 打印内存
// 打印内存
for i in (0..0x200).step_by(16) {
    info!("{:#05x}  {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}", 
        i, buf[i], buf[i+1],buf[i+2], buf[i+3],buf[i+4], buf[i+5],buf[i+6], buf[i+7], 
        buf[i+8], buf[i+9],buf[i+10], buf[i+11],buf[i+12], buf[i+13],buf[i+14], buf[i+15]);
}
```

### 2. 打印文件树

```rust
pub extern "C" fn rust_main(hartid: usize, device_tree_paddr: usize) -> ! {
    ...
    // 输出文件树
    print_file_tree(FILETREE.lock().open("/").unwrap());
    ...
}
// 打印目录树
pub fn print_file_tree(node: FileTreeNode) {
    // info!("is root {:?}", node.is_root());
    info!("{}", node.get_pwd());
    print_file_tree_back(&node, 0);
}

// 打印目录树 - 递归
pub fn print_file_tree_back(node: &FileTreeNode, space: usize) {
    let iter = node.get_children();
    let mut iter = iter.iter().peekable();
    while let Some(sub_node) = iter.next() {
        if iter.peek().is_none() {
            info!("{:>2$}└──{}", "", sub_node.get_filename(), space);
        } else {
            info!("{:>2$}├──{}", "", sub_node.get_filename(), space);
        }
        if sub_node.is_dir() {
            print_file_tree_back(sub_node, space + 3);
        }
    }
}
```

### 3. 输出文件信息

```rust
// 测试读取文件
match FILETREE.lock().open("text.txt") {
    Ok(file_txt) => {
        let file_txt = file_txt.to_file();
        let file_txt_content = file_txt.read();
        info!("读取到内容: {}", file_txt.size);
        info!("文件内容：{}", String::from_utf8_lossy(&file_txt_content));
    }
    Err(err) => {
        info!("读取文件错误: {}", &err);
    }
};
```

## 比赛收获

这次全国大学生操作系统大赛中，我们收获了很多，不论是比赛的创新意识还是有始有终、竭尽所能的态度，又或是团队协作的精神，都将是人生的财富。
(一)留心生活，寻找灵感
  在这场比赛中最值得学习还是创意，好的创意等于成功的一半。想法往往来自于生活的细节，想要找到一个好创意，首先要先浏览以往的操作系统赛事，从中找到灵感。然后在现实世界中添加更多创新元素，使其更好地服务于社会。

(二)坚定不移，用心准备
  “机会总是留给那些有准备的人”面对机遇我们一直在准备，我们在这场持续三个多月的操作系统比赛中遇到了很多困难。但是在困难面前我们没有屈服,通过我们不断的努力，将众多困难一一克服。面对操作系统中的许多问题，我们经常在深夜讨论，查阅资料，并积极咨询辅导老师。为了清楚地规划数据，我们做了很多实际的调查，并请求导师做指导。

(三)注重团队合作
  全国大学生操作系统大赛，这不是个人的比赛，是整个团队的竞争。团队精神是非常重要的，团队是成功的保障，是合作精神和服务精神的集中体现。胜出的团队应该是一个拥有全面技能的团队，包括技术和管理人员。每个团队成员都可以灵活，协调和有效地工作，并且可以相互信任，互相帮助并面对问题，一起战胜。

## MUSL LIBC

### TLS

[c - On Linux, is TLS set up by the kernel or by libc (or other language runtime)? - Stack Overflow](https://stackoverflow.com/questions/30377020/on-linux-is-tls-set-up-by-the-kernel-or-by-libc-or-other-language-runtime)

[linux - ELF file TLS and LOAD program sections - Stack Overflow](https://stackoverflow.com/questions/4126184/elf-file-tls-and-load-program-sections)

https://stackoverflow.com/questions/64957077/get-argv0-from-a-none-main-file-such-as-start-no-libc-no-libs/64957497#64957497

The ELF entry point contract is **not a C function** in the vast majority of psABIs (processor-specific ABI, the arch-dependent part of ELF). At entry, the stack pointer register points to an array of system-word-sized slots consisting of:

```c
argc, argv[0], ..., argv[argc-1], 0, environ[0], ..., environ[N], 0,
auxv[0].a_type, auxv[0].a_value, ..., 0
```

You need at least a minimal asm stub to convert this into form that's readable by C. The simplest way to do this is to copy the stack pointer register into the first-argument register, align the stack pointer down according the function call ABI requirements, and call your C function taking a single pointer argument.

You can see the `crt_arch.h` files ([x86_64 version here](https://git.musl-libc.org/cgit/musl/tree/arch/x86_64/crt_arch.h?id=v1.2.1)) in musl libc for an example of how this is done. (You can probably ignore the part about `_DYNAMIC` which is arranging for self-relocation when the entry point is used in the dynamic linker startup or static PIE executables.)
