# 10. 内存操作的事务性与失败回滚

## 一次映射不一定只有一步

为一个虚拟 Page 建立映射，可能需要：

```text
1. 预留虚拟地址
2. 分配数据页帧
3. 可能分配若干中间页表页
4. 写入页表项
5. 刷新 TLB
```

任何一步都可能失败。例如物理内存不足、虚拟地址冲突、映射已经存在或分配器返回错误。如果第 3 步失败，而第 2 步申请的数据页帧没有释放，就会造成资源泄漏。

## 事务式思路

把多个底层操作当作一个整体：

- 全部成功：提交结果。
- 任一步失败：撤销之前已经完成的操作，恢复到调用前的状态。

简化伪代码：

```rust
let page = virtual_allocator.allocate()?;
let frame = match frame_allocator.allocate() {
    Some(frame) => frame,
    None => {
        virtual_allocator.deallocate(page);
        return Err(OutOfMemory);
    }
};

match mapper.map(page, frame) {
    Ok(flush) => {
        flush.flush();
        Ok(page)
    }
    Err(error) => {
        frame_allocator.deallocate(frame);
        virtual_allocator.deallocate(page);
        Err(error.into())
    }
}
```

这只是简化示意。真实实现还要考虑 `map` 失败前是否已经创建中间页表、这些页表页由谁拥有、是否需要清理，以及 flush 是否已执行。

## 回滚顺序

回滚通常按资源依赖关系逆序执行：

- 如果映射已建立，先撤销映射。
- 确认 TLB 不再引用旧映射。
- 释放本次操作独占的数据页帧。
- 释放本次操作新建且现在为空的中间页表页。
- 释放虚拟地址预留。

不能释放仍被其他映射或地址空间使用的物理页帧。

## RAII 与资源守卫

Rust 可以用 RAII 守卫表达“资源尚未提交”的状态。守卫在 `Drop` 中执行清理；全部步骤成功后调用 `commit()`，让守卫不再回滚。这能减少遗漏清理的分支，但 `Drop` 中不能随意 panic，也必须考虑哪些操作需要 `unsafe`。

## 思考题

1. 如果物理页帧申请成功，但建立映射失败，应该释放什么？
2. 为什么回滚顺序通常与资源申请顺序相反？
3. `unmap` 与释放物理页帧为什么要分开？
4. 一个 RAII 守卫何时应该提交？

## 本课结论

可靠的内存管理不只要考虑成功路径，还必须把失败路径设计成一等公民。每项资源都需要明确的所有权、提交点和回滚策略。


## 代码演示：把映射流程拆成可回滚的阶段

**来源标记：按历史课程中的失败回滚讨论重构。** 本节代码是流程伪代码，重点是资源所有权和错误分支；真实的 x86_64 Mapper API 见第 8 章及 labs/kernel-memory。

~~~rust
// 阶段 1：预留一个虚拟页。
// 此时只占用了虚拟地址空间分配器的元数据，还没有物理页帧。
let page = virtual_allocator.allocate()?;

// 阶段 2：申请一个物理数据页帧。
let frame = match frame_allocator.allocate_frame() {
    Some(frame) => frame,
    None => {
        // 物理页帧申请失败，释放之前预留的虚拟页。
        virtual_allocator.deallocate(page);
        return Err(OutOfMemory);
    }
};

// 阶段 3：建立页表映射。
// 实际 x86_64 API 的核心形式为：
// unsafe { mapper.map_to(page, frame, flags, frame_allocator) }

match mapper.map(page, frame) {
    Ok(flush) => {
        // 先让 CPU 不再使用旧的地址转换缓存。
        flush.flush();

        // 所有步骤成功，提交这次分配。
        Ok(page)
    }
    Err(error) => {
        // 只有确认数据帧没有成功映射、没有被共享时，
        // 才能把它归还给物理页帧分配器。
        unsafe {
            frame_allocator.deallocate_frame(frame);
        }

        // 只有确认虚拟页没有遗留映射后，才能释放 VA 预留。
        virtual_allocator.deallocate(page);
        Err(error.into())
    }
}
~~~

### 逐步解释

1. **预留 VA**：避免两个分配请求同时拿到同一虚拟区间。此时并没有因此消耗同样大小的物理 RAM。
2. **分配 PA**：物理页帧分配器把一个 Frame 的所有权交给调用者。此时还没有自动建立 VA → PA 映射。
3. **建立映射**：Mapper 可能还要从 FrameAllocator 申请中间页表页。四级分页下，首次建立某个分支最多可能需要额外的 PDPT、PD 和 PT 页帧。
4. **刷新 TLB**：页表改动后必须让相关 CPU 不再使用过期的地址转换缓存。
5. **提交**：只有映射和 flush 成功，才把这次 VA 分配视为完成。

### 失败回滚不能只写两行 deallocate

上面的流程故意简化了一个关键问题：Mapper 在失败前是否已经创建了部分中间页表？如果创建过，这些页表页由谁负责回收？因此，不能把伪代码直接复制成生产级内存管理器。

真实实现需要记录每个阶段创建的资源，并按逆序回滚：

~~~text
申请 VA
  |
申请数据 Frame
  |
创建中间页表（可能为 0 到 3 张）
  |
安装最终 PTE
  |
flush
  |
commit
~~~

失败时，要先确认是否存在已安装的映射；如果存在，先撤销映射并 flush；再回收本次操作独占的数据 Frame；再回收本次新建且已经为空的中间页表；最后释放 VA 预留。不能释放其他地址空间仍然共享的页帧。

### 为什么 deallocate_frame 是 unsafe

x86_64 crate 的 FrameDeallocator 将 deallocate_frame 定义为 unsafe：调用者必须保证传入页帧确实没有在使用。重复释放或释放仍被映射的页帧，会让同一个物理内存同时分配给多个对象。

### 当前 RustOS 的实现边界

当前仓库提供了 Free List 和 Mapper 的可编译模块，但还没有把 VA allocator、Frame allocator、Mapper、TLB flush、失败回滚和共享页生命周期全部封装为统一 MemoryManager。历史第 9 课实验也正是因为需要整合这些部分而暂缓。
