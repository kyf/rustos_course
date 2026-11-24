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
