#[derive(Debug)]
pub enum RuntimeError {
    NoEnoughPage,
    FileNotFound,
    // 没有对应的物理地址
    NoMatchedAddr,
    ChangeTask,
    // 没有对应的文件
    NoMatchedFile,
    // 没有对应的fd
    NoMatchedFileDesc,
    // 杀死当前任务
    KillCurrentTask,
    // 杀死当前进程,
    KillCurrentProc,
    // 触发EBADF
    EBADF,
    // 信号返回程序
    SigReturn,
    //
    WriteZero,
    UnexpectedEof,
    NotRWFile,
    NotDir
}