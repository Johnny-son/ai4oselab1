//! 处理器管理模块
//!
//! 定义 `PROCESSOR` 全局变量和 `ProcManager` 进程管理器。
//!
//! ## 设计思路
//!
//! 进程管理分为两部分：
//! - `PROCESSOR`：封装 `PManager`，提供全局访问接口，管理当前运行的进程
//! - `ProcManager`：实现 `Manage` 和 `Schedule` trait，负责进程的存储和调度
//!
//! ## 调度算法
//!
//! 本章实现 **stride 调度算法**：
//! - 每个进程维护一个 stride（累计步长）和 priority（优先级）
//! - 每次选择 stride 最小的进程运行，并将其 stride 增加 `BIG_STRIDE / priority`
//! - 进程优先级越高（priority 越大），stride 增长越慢，从而获得更多调度机会。

use crate::process::Process;
use alloc::collections::{BTreeMap, BinaryHeap};
use core::{
    cell::UnsafeCell,
    cmp::{Ordering, Reverse},
};
use tg_task_manage::{Manage, PManager, ProcId, Schedule};

/// 处理器全局管理器
///
/// 封装 `PManager<Process, ProcManager>`，通过 `UnsafeCell` 提供内部可变性。
/// 在单核环境下是安全的，因为不会出现并发访问。
pub struct Processor {
    inner: UnsafeCell<PManager<Process, ProcManager>>,
}

unsafe impl Sync for Processor {}

impl Processor {
    /// 创建新的处理器管理器（编译期常量初始化）
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(PManager::new()),
        }
    }

    /// 获取内部 PManager 的可变引用
    #[inline]
    pub fn get_mut(&self) -> &mut PManager<Process, ProcManager> {
        unsafe { &mut (*self.inner.get()) }
    }
}

/// 全局处理器管理器实例
pub static PROCESSOR: Processor = Processor::new();

/// 进程管理器
///
/// 负责管理所有进程实体和调度队列：
/// - `tasks`：以 ProcId 为键的进程映射表，存储所有进程实体
/// - `ready_queue`：按照 stride 排序的最小堆，就绪进程会根据优先级被公平调度
pub struct ProcManager {
    /// 所有进程实体的映射表
    tasks: BTreeMap<ProcId, Process>,
    /// 就绪队列，使用最小堆按 stride 排序
    ready_queue: BinaryHeap<Reverse<ReadyEntry>>,
    /// 用于打破 stride 相等时的调度顺序
    sequence: usize,
}

impl ProcManager {
    /// 创建新的进程管理器
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            ready_queue: BinaryHeap::new(),
            sequence: 0,
        }
    }

    /// 将指定进程加入就绪队列
    fn push_ready(&mut self, id: ProcId) {
        if let Some(task) = self.tasks.get(&id) {
            let entry = ReadyEntry {
                stride: task.stride,
                order: self.sequence,
                pid: id,
            };
            self.sequence = self.sequence.wrapping_add(1);
            self.ready_queue.push(Reverse(entry));
        }
    }
}

/// 实现 Manage trait：进程实体的增删查
impl Manage<Process, ProcId> for ProcManager {
    /// 插入新进程到进程表
    #[inline]
    fn insert(&mut self, id: ProcId, task: Process) {
        self.tasks.insert(id, task);
    }

    /// 根据 PID 获取进程的可变引用
    #[inline]
    fn get_mut(&mut self, id: ProcId) -> Option<&mut Process> {
        self.tasks.get_mut(&id)
    }

    /// 从进程表中删除进程（回收资源）
    #[inline]
    fn delete(&mut self, id: ProcId) {
        self.tasks.remove(&id);
    }
}

/// 实现 Schedule trait：进程调度（当前为 FIFO/RR）
impl Schedule<ProcId> for ProcManager {
    /// 将进程加入就绪队列尾部
    fn add(&mut self, id: ProcId) {
        self.push_ready(id);
    }

    /// 从就绪队列头部取出下一个要执行的进程
    fn fetch(&mut self) -> Option<ProcId> {
        while let Some(Reverse(entry)) = self.ready_queue.pop() {
            if let Some(task) = self.tasks.get_mut(&entry.pid) {
                let step = task.stride_step();
                task.stride = task.stride.saturating_add(step);
                return Some(entry.pid);
            }
        }
        None
    }
}

/// 就绪队列中的条目
#[derive(Eq, Clone, Copy)]
struct ReadyEntry {
    stride: i64,
    order: usize,
    pid: ProcId,
}

impl PartialEq for ReadyEntry {
    fn eq(&self, other: &Self) -> bool {
        self.stride == other.stride && self.order == other.order && self.pid == other.pid
    }
}

impl PartialOrd for ReadyEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReadyEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.stride.cmp(&other.stride) {
            Ordering::Equal => match self.order.cmp(&other.order) {
                Ordering::Equal => self.pid.cmp(&other.pid),
                ord => ord,
            },
            ord => ord,
        }
    }
}
