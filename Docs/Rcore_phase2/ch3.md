# RCore phase2 ch3
要求：实现系统调用syscall_trace
实现思路：
添加syscall_trace的id，实现syscall_trace
```
/// sys_trace 系统调用
/// - request = 0 (Read): 读取地址 id 处的一个字节，返回该字节的值
/// - request = 1 (Write): 向地址 id 写入 data 的低8位，成功返回0
/// - request = 2 (Syscall): 返回系统调用 id 的调用次数
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace request={}, id={:#x}, data={}", trace_request, id, data);
    match trace_request {
        TRACE_READ => {
            // 从用户态地址读取一个字节
            let addr = id as *const u8;
            unsafe { (*addr) as isize }
        }
        TRACE_WRITE => {
            // 向用户态地址写入一个字节
            let addr = id as *mut u8;
            unsafe {
                *addr = data as u8;
            }
            0
        }
        TRACE_SYSCALL => {
            // 返回系统调用的调用次数
            get_syscall_times(id) as isize
        }
        _ => -1,
    }
}
```
然后分配一个大数组来记录每个syscall的调用次数
当系统进行调用时，对应索引位置count++
主要功能函数：
```
/// 更新当前任务的系统调用计数
pub fn update_syscall_times(syscall_id: usize) {
    if syscall_id < MAX_SYSCALL_NUM {
        let mut inner = TASK_MANAGER.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[syscall_id] += 1;
    }
}

/// 获取当前任务某个系统调用的调用次数
pub fn get_syscall_times(syscall_id: usize) -> u32 {
    if syscall_id < MAX_SYSCALL_NUM {
        let inner = TASK_MANAGER.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[syscall_id]
    } else {
        0
    }
}
```
优化思考
目前实现的syscall较少，一个大数组，内容基本是空的，因此可以用一个哈希表来存储系统调用
已经实现通过测试