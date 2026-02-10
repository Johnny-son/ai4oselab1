//! 任务管理模块
//!
//! 定义了任务控制块（Task Control Block, TCB）和调度事件，
//! 是多道程序系统的核心数据结构。
//!
//! ## 与第二章的区别
//!
//! 第二章的批处理系统中，用户上下文直接在 `rust_main` 的局部变量中管理。
//! 本章将其封装到 `TaskControlBlock` 中，每个任务拥有独立的 TCB，
//! 包含用户上下文、完成状态和独立的用户栈，支持多任务并发。

use tg_kernel_context::LocalContext;
use tg_syscall::{Caller, SyscallId};

/// 系统调用计数数组的最大长度（覆盖所有可能的系统调用号，最大为 410+1）
const SYSCALL_COUNT_MAX: usize = 512;

/// 任务控制块（Task Control Block, TCB）
///
/// 每个用户程序对应一个 TCB，包含：
/// - `ctx`：用户态上下文（所有通用寄存器 + 控制寄存器），用于任务切换时保存/恢复状态
/// - `finish`：任务是否已完成（退出或被杀死）
/// - `stack`：用户栈空间（8 KiB），每个任务有独立的栈
/// - `syscall_counts`：系统调用计数数组，按系统调用号索引
pub struct TaskControlBlock {
    /// 用户态上下文：保存 Trap 时的所有寄存器状态
    ctx: LocalContext,
    /// 任务完成标志：true 表示已退出或被杀死
    pub finish: bool,
    /// 用户栈：8 KiB（1024 个 usize = 1024 × 8 = 8192 字节）
    /// 每个任务拥有独立的栈空间，避免栈溢出影响其他任务
    stack: [usize; 1024],
    /// 系统调用计数数组：记录每个系统调用的调用次数
    syscall_counts: [usize; SYSCALL_COUNT_MAX],
}

/// 调度事件
///
/// `handle_syscall` 处理完系统调用后返回此枚举，
/// 告知主循环应该如何调度当前任务。
pub enum SchedulingEvent {
    /// 系统调用处理完成，继续执行当前任务（如 write、clock_gettime）
    None,
    /// 任务主动让出 CPU（yield 系统调用），切换到下一个任务
    Yield,
    /// 任务请求退出（exit 系统调用），附带退出码
    Exit(usize),
    /// 不支持的系统调用，附带系统调用 ID
    UnsupportedSyscall(SyscallId),
}

impl TaskControlBlock {
    /// 零值常量：用于数组初始化
    pub const ZERO: Self = Self {
        ctx: LocalContext::empty(),
        finish: false,
        stack: [0; 1024],
        syscall_counts: [0; SYSCALL_COUNT_MAX],
    };

    /// 初始化一个任务
    ///
    /// - 清零用户栈
    /// - 创建用户态上下文，设置入口地址和 `sstatus.SPP = User`
    /// - 将栈指针设置为用户栈的栈顶（高地址端）
    pub fn init(&mut self, entry: usize) {
        self.stack.fill(0);
        self.finish = false;
        self.syscall_counts.fill(0);
        self.ctx = LocalContext::user(entry);
        // 栈从高地址向低地址增长，所以 sp 指向栈顶（数组末尾之后的地址）
        *self.ctx.sp_mut() = self.stack.as_ptr() as usize + core::mem::size_of_val(&self.stack);
    }

    /// 执行此任务
    ///
    /// 恢复用户寄存器并执行 `sret` 切换到 U-mode。
    /// 当用户程序触发 Trap 后返回到此函数的调用处。
    #[inline]
    pub unsafe fn execute(&mut self) {
        unsafe { self.ctx.execute() };
    }

    /// 处理系统调用，返回调度事件
    ///
    /// 从用户上下文中提取系统调用 ID（a7 寄存器）和参数（a0-a5 寄存器），
    /// 分发到对应的处理函数，并将返回值写回 a0 寄存器。
    pub fn handle_syscall(&mut self) -> SchedulingEvent {
        use tg_syscall::{SyscallId as Id, SyscallResult as Ret};
        use SchedulingEvent as Event;

        // a7 寄存器存放 syscall ID
        let id: SyscallId = self.ctx.a(7).into();
        // a0-a5 寄存器存放系统调用参数
        let args = [
            self.ctx.a(0),
            self.ctx.a(1),
            self.ctx.a(2),
            self.ctx.a(3),
            self.ctx.a(4),
            self.ctx.a(5),
        ];

        // 统计系统调用次数：在处理之前先计数
        let id_val: usize = id.0;
        if id_val < SYSCALL_COUNT_MAX {
            self.syscall_counts[id_val] += 1;
        }

        // 通过 Caller.entity 传递 syscall_counts 数组的指针，
        // 使得 Trace 实现可以访问当前任务的系统调用计数
        let caller = Caller {
            entity: self.syscall_counts.as_ptr() as usize,
            flow: 0,
        };
        match tg_syscall::handle(caller, id, args) {
            Ret::Done(ret) => match id {
                // exit 系统调用：返回退出事件
                Id::EXIT => Event::Exit(self.ctx.a(0)),
                // yield 系统调用：返回让出事件
                Id::SCHED_YIELD => {
                    *self.ctx.a_mut(0) = ret as _;
                    self.ctx.move_next(); // sepc += 4，跳过 ecall 指令
                    Event::Yield
                }
                // 其他系统调用（如 write、clock_gettime）：继续执行
                _ => {
                    *self.ctx.a_mut(0) = ret as _;
                    self.ctx.move_next(); // sepc += 4，跳过 ecall 指令
                    Event::None
                }
            },
            // 不支持的系统调用
            Ret::Unsupported(_) => Event::UnsupportedSyscall(id),
        }
    }
}
