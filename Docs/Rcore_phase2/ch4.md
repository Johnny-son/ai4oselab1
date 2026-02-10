# RCore phase2 ch4 

## 1.任务要求

本阶段主要要实现四个系统调用：

1. `sys_get_time`：把当前时间写回用户传入的 `TimeVal*`。
2. `sys_trace`：支持 Read/Write/Syscall 三类请求。
3. `sys_mmap`：在当前进程地址空间新增一段映射（按 `prot` 设置权限）。
4. `sys_munmap`：删除映射并回收页帧，要求支持部分取消映射（可能会把一个区域切成两段）。

同时为了 `trace(Syscall)` 统计次数，需要维护每个任务的 syscall 调用计数。


## 2. 总体设计（模块怎么串起来）

我把 ch4 的新增逻辑拆成 3 条链路：

### 2.1 用户指针安全访问（基础设施）

- 位置：`os/src/mm/page_table.rs`
- 作用：给 `get_time`/`trace` 提供“可检查权限的用户内存访问”。

### 2.2 syscall 实现（调用基础设施 + 操作地址空间）

- 位置：`os/src/syscall/process.rs`
- 作用：在 syscall 层做参数检查，然后：
  - `get_time/trace` 调用“用户指针安全访问”函数
  - `mmap/munmap` 调用 task 层封装的 `current_mmap/current_munmap`


## 3. 用户指针安全访问：为什么要做？怎么做？

### 3.1 ch3 的写法为什么不行？

ch3 的 `sys_trace` 可以这么写：

- Read：`unsafe { *(id as *const u8) }`
- Write：`unsafe { *(id as *mut u8) = data as u8 }`

但 ch4 引入虚拟内存后，这里的 `id` 是**用户虚拟地址**：

- 内核不能直接解引用用户虚拟地址
- 必须通过“当前进程的页表”翻译到物理地址
- 必须检查 PTE 权限位：`U/R/W`

### 3.2 新增的几个关键函数

下面这些函数在 `os/src/mm/page_table.rs`：

#### (1) `PageTable::translate_va(va)`

```rust
/// Translate a virtual address to (physical page number, page offset, pte flags)
/// Return None if the mapping does not exist.
pub fn translate_va(&self, va: VirtAddr) -> Option<(PhysPageNum, usize, PTEFlags)> {
   let vpn = va.floor();
   let offset = va.page_offset();
   self.translate(vpn)
      .map(|pte| (pte.ppn(), offset, pte.flags()))
}
```

#### (2) `translated_read_u8(token, ptr)`

从用户虚拟地址读 1 字节，要求权限：

- 必须含 `U`（用户可访问）
- 必须含 `R`（可读）

失败返回 `None`。

对应代码（`os/src/mm/page_table.rs`）：

```rust
/// Read a byte from user virtual address with permission check.
pub fn translated_read_u8(token: usize, ptr: *const u8) -> Option<u8> {
   let page_table = PageTable::from_token(token);
   let va = VirtAddr::from(ptr as usize);
   let (ppn, off, flags) = page_table.translate_va(va)?;
   if !flags.contains(PTEFlags::U) || !flags.contains(PTEFlags::R) {
      return None;
   }
   Some(ppn.get_bytes_array()[off])
}
```

#### (3) `translated_write_u8(token, ptr, val)`

向用户虚拟地址写 1 字节，要求权限：

- 必须含 `U`
- 必须含 `W`

失败返回 `Err(())`。

对应代码（`os/src/mm/page_table.rs`）：

```rust
/// Write a byte to user virtual address with permission check.
pub fn translated_write_u8(token: usize, ptr: *mut u8, val: u8) -> Result<(), ()> {
   let page_table = PageTable::from_token(token);
   let va = VirtAddr::from(ptr as usize);
   let (ppn, off, flags) = page_table.translate_va(va).ok_or(())?;
   if !flags.contains(PTEFlags::U) || !flags.contains(PTEFlags::W) {
      return Err(());
   }
   ppn.get_bytes_array()[off] = val;
   Ok(())
}
```

#### (4) `copy_to_user(token, dst, src)`

把一段内核 buffer 拷贝到用户虚拟地址，特点：

- **逐页检查**：每页都要 `U|W`
- 支持跨页：比如 `TimeVal` 被拆在两页里也能写对

它做法大概是：

- while 还有没写完：
  - 翻译当前 `dst` 所在页
  - 算本页还能写多少（`PAGE_SIZE - page_off`）
  - 拷贝一段，推进指针

对应代码（`os/src/mm/page_table.rs`，核心循环）：

```rust
/// Copy bytes from kernel buffer to user virtual memory with permission check (U|W per page).
pub fn copy_to_user(token: usize, dst: *mut u8, src: &[u8]) -> Result<(), ()> {
   let page_table = PageTable::from_token(token);
   let mut start = dst as usize;
   let end = start + src.len();
   let mut copied = 0usize;
   while start < end {
      let start_va = VirtAddr::from(start);
      let vpn = start_va.floor();
      let pte = page_table.translate(vpn).ok_or(())?;
      let flags = pte.flags();
      if !flags.contains(PTEFlags::U) || !flags.contains(PTEFlags::W) {
         return Err(());
      }
      let ppn = pte.ppn();
      let page_off = start_va.page_offset();
      let page_left = PAGE_SIZE - page_off;
      let to_copy = (end - start).min(page_left);
      let dst_slice = &mut ppn.get_bytes_array()[page_off..page_off + to_copy];
      dst_slice.copy_from_slice(&src[copied..copied + to_copy]);
      start += to_copy;
      copied += to_copy;
   }
   Ok(())
}
```

## 4. `sys_get_time`：如何保证跨页也正确？

- 位置：`os/src/syscall/process.rs`
- 关键点：不能直接写 `*ts = tv`，必须用 `copy_to_user`。

实现思路：

1. `get_time_us()` 取到微秒
2. 构造 `TimeVal { sec, usec }`
3. 把 `TimeVal` 看成字节切片（`&[u8]`）
4. `copy_to_user(current_user_token(), ts as *mut u8, bytes)`

写成功返回 0，失败返回 -1。

对应代码（`os/src/syscall/process.rs`）：

```rust
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
   trace!("kernel: sys_get_time");
   let us = get_time_us();
   let tv = TimeVal {
      sec: us / 1_000_000,
      usec: us % 1_000_000,
   };
   // 通过逐字节拷贝，天然支持跨页
   let bytes = unsafe {
      core::slice::from_raw_parts(
         (&tv as *const TimeVal) as *const u8,
         core::mem::size_of::<TimeVal>(),
      )
   };
   match copy_to_user(current_user_token(), ts as *mut u8, bytes) {
      Ok(()) => 0,
      Err(()) => -1,
   }
}
```
---

## 5. `MemorySet::mmap_area/munmap_area`：地址空间层如何实现？

- 位置：`os/src/mm/memory_set.rs`

### 5.1 `is_range_free`：冲突检测

把 `[start, end)` 转成 VPN 范围，检查是否与任何已有 `MapArea` 重叠。

对应代码（`os/src/mm/memory_set.rs`）：

```rust
/// Check whether [start, end) (in VPN) overlaps with any existing area.
pub fn is_range_free(&self, start: VirtAddr, end: VirtAddr) -> bool {
   let start_vpn = start.floor();
   let end_vpn = end.ceil();
   self.areas.iter().all(|area| {
      let a_start = area.vpn_range.get_start();
      let a_end = area.vpn_range.get_end();
      end_vpn <= a_start || start_vpn >= a_end
   })
}
```

如果重叠，`mmap_area` 返回 false。

### 5.2 `mmap_area`：插入 framed area

- 先检查 start<end 且区间空闲
- `push(MapArea::new(..., MapType::Framed, perm))`
- `push` 内部会 `map_area.map(&mut page_table)`，为每个页分配 frame 并建立映射

对应代码（`os/src/mm/memory_set.rs`）：

```rust
/// Map a new framed area into this address space. Return false if overlaps.
pub fn mmap_area(&mut self, start: VirtAddr, end: VirtAddr, perm: MapPermission) -> bool {
   if start >= end {
      return false;
   }
   if !self.is_range_free(start, end) {
      return false;
   }
   self.push(MapArea::new(start, end, MapType::Framed, perm), None);
   true
}
```

### 5.3 `munmap_area`：支持 shrink/split（核心难点）

目标：取消映射 `[start,end)`，并支持“只取消其中一段”。

对每个可能重叠的 `MapArea`，分 4 类情况：

1. **完全覆盖**：直接 remove 该 area，并 `area.unmap(&mut page_table)`
2. **从左侧削掉**（start <= area.start && end < area.end）：调用 `shrink_left(end_vpn)`
3. **从右侧削掉**（start > area.start && end >= area.end）：调用 `shrink_to(start_vpn)`
4. **中间挖洞**：需要 split 成左右两段：
   - 先把原 area shrink 成左段 `[a_start, start_vpn)`
   - 再创建一个右段 `[end_vpn, a_end)` 的新 `MapArea`，并 map

最终只要触碰到了至少一段 area，就返回 true；如果区间完全不在任何映射内，返回 false。

对应代码（`os/src/mm/memory_set.rs`，核心逻辑）：

```rust
/// Unmap range [start, end) from this address space.
/// Support partial unmap by shrinking/splitting areas.
pub fn munmap_area(&mut self, start: VirtAddr, end: VirtAddr) -> bool {
   if start >= end {
      return false;
   }
   let start_vpn = start.floor();
   let end_vpn = end.ceil();

   let mut i = 0usize;
   let mut touched = false;
   while i < self.areas.len() {
      let a_start = self.areas[i].vpn_range.get_start();
      let a_end = self.areas[i].vpn_range.get_end();
      if end_vpn <= a_start || start_vpn >= a_end {
         i += 1;
         continue;
      }
      touched = true;
      // overlap exists
      if start_vpn <= a_start && end_vpn >= a_end {
         // remove whole area
         let mut area = self.areas.remove(i);
         area.unmap(&mut self.page_table);
         continue;
      }
      if start_vpn <= a_start && end_vpn < a_end {
         // shrink from left
         self.areas[i].shrink_left(&mut self.page_table, end_vpn);
         i += 1;
         continue;
      }
      if start_vpn > a_start && end_vpn >= a_end {
         // shrink from right
         self.areas[i].shrink_to(&mut self.page_table, start_vpn);
         i += 1;
         continue;
      }
      // split into two areas
      let right_start = end_vpn;
      let right_end = a_end;
      let right_map_type = self.areas[i].map_type;
      let right_map_perm = self.areas[i].map_perm;
      // left part shrink
      self.areas[i].shrink_to(&mut self.page_table, start_vpn);
      // create right part and map
      let r_start_va: VirtAddr = right_start.into();
      let r_end_va: VirtAddr = right_end.into();
      let mut right = MapArea::new(r_start_va, r_end_va, right_map_type, right_map_perm);
      right.map(&mut self.page_table);
      self.areas.push(right);
      i += 1;
   }
   touched
}
```

另外，为了给 syscall 层一个“好用的入口”，task 层额外封装了：

```rust
/// Map a new area into current task's address space.
pub fn current_mmap(start: crate::mm::VirtAddr, end: crate::mm::VirtAddr, perm: crate::mm::MapPermission) -> bool {
   let mut inner = TASK_MANAGER.inner.exclusive_access();
   let cur = inner.current_task;
   inner.tasks[cur].memory_set.mmap_area(start, end, perm)
}

/// Unmap an area from current task's address space.
pub fn current_munmap(start: crate::mm::VirtAddr, end: crate::mm::VirtAddr) -> bool {
   let mut inner = TASK_MANAGER.inner.exclusive_access();
   let cur = inner.current_task;
   inner.tasks[cur].memory_set.munmap_area(start, end)
}
```
