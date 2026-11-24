# 08. 建立和撤销虚拟内存映射

## 映射关系

一条基础页映射可以表示为：

```text
Virtual Page 0x4000_0000 -> Physical Frame 0x0200_0000
```

页表项除了保存物理页帧地址，还包含权限和状态标志，例如 PRESENT、WRITABLE、USER_ACCESSIBLE、NO_EXECUTE。权限需要沿页表遍历路径共同判断，不能只看最后一级 PTE。

## map_to 的概念流程

使用 `x86_64` crate 的 Mapper 抽象时，`map_to` 的概念工作包括：

1. 确认目标虚拟 Page 和物理 Frame 满足对齐。
2. 从 PML4 开始遍历页表。
3. 如果下级页表不存在，申请新的页帧作为页表。
4. 初始化新页表并建立上级页表项。
5. 在最后一级写入目标物理页帧和权限标志。
6. 返回刷新句柄或要求调用者刷新 TLB。

实际 API 的参数、错误类型和 flush 返回值会随 crate 版本变化，请以项目锁定版本为准。

## 为什么映射一个页面可能需要多个页帧

除了被映射的数据页帧，页表树本身也可能需要新的页帧。例如一个虚拟地址第一次落在某个 PML4 分支下，可能需要新建 PDPT、PD、PT。之后同一分支下的其他映射可以复用这些页表页。

因此映射操作可能同时涉及一个数据页帧和零个或多个中间页表页帧。

## unmap 的概念流程

1. 找到目标虚拟 Page 的页表项。
2. 确认该映射存在。
3. 移除最后一级映射。
4. 获取原来映射的物理 Frame。
5. 刷新相应 TLB 项。
6. 根据页表树是否为空，决定是否回收中间页表页。
7. 由明确的所有者决定数据页帧何时释放。

**解除映射不等于释放物理页帧。** 一个页帧可能被多个虚拟地址或多个进程共享。

## 所有权要明确

设计 API 时，需要回答：谁申请数据页帧？谁拥有它？是否被多个地址空间共享？`unmap` 返回页帧后由谁释放？中间页表页由哪个地址空间拥有？内核共享页表是否允许某个进程销毁时回收？

这些问题决定是否会出现泄漏、重复释放或释放仍在使用的内存。

## 思考题

1. 映射一个 VA → PA 时，为什么可能需要额外分配页表页？
2. `unmap` 是否应该自动释放数据页帧？为什么？
3. 页表项已经修改，CPU 是否一定立即看到新映射？
4. 共享物理页帧需要怎样管理生命周期？

## 本课结论

映射是“页表结构修改 + 物理页帧所有权管理 + TLB 一致性”的组合操作。只改一个 PTE 并不足以构成完整、安全的内存管理接口。


## 代码演示：OffsetPageTable 上建立一个 4 KiB 映射

**来源标记：按历史课程中的 Mapper / map_to / flush 讨论重构，API 对齐 x86_64 0.15.5。**

\`\`\`rust
use x86_64::VirtAddr;
use x86_64::structures::paging::{
    Mapper,
    OffsetPageTable,
    Page,
    PageTableFlags,
    PhysFrame,
    Size4KiB,
};

use crate::memory::FreeListFrameAllocator;

/// 把一个虚拟页映射到指定物理页帧。
///
/// # Safety
/// 调用者必须确保：虚拟页当前没有被其他对象使用；
/// frame 已由调用者独占分配；映射权限符合内核安全策略。
pub unsafe fn map_4k(
    mapper: &mut OffsetPageTable<'static>,
    allocator: &mut FreeListFrameAllocator,
    virtual_address: VirtAddr,
    frame: PhysFrame<Size4KiB>,
) -> Result<(), x86_64::structures::paging::mapper::MapToError<Size4KiB>> {
    let page = Page::containing_address(virtual_address);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    let flush = unsafe {
        mapper.map_to(page, frame, flags, allocator)?
    };

    // 页表已更新，要求 CPU 使对应的旧地址转换缓存失效。
    flush.flush();

    Ok(())
}
\`\`\`

### 为什么 map_to 需要 FrameAllocator 参数？

如果目标虚拟地址所在的页表分支还没有创建，Mapper 需要额外分配中间页表页。四级分页最坏情况下需要新建 PDPT、PD 和 PT 三张表，因此文档说明最多可能额外分配三个 4 KiB 页帧。传入的 allocator 为这些页表页提供物理内存。

这也说明：映射一个数据页，不一定只消耗一个物理页帧。

### 代码演示：解除映射，但暂不释放物理页帧

\`\`\`rust
use x86_64::structures::paging::{
    Mapper, OffsetPageTable, Page, PhysFrame, Size4KiB,
};
use x86_64::VirtAddr;

pub fn unmap_4k(
    mapper: &mut OffsetPageTable<'static>,
    virtual_address: VirtAddr,
) -> Result<PhysFrame<Size4KiB>, x86_64::structures::paging::mapper::UnmapError> {
    let page = Page::containing_address(virtual_address);

    let (frame, flush) = mapper.unmap(page)?;
    flush.flush();

    // 返回物理页帧，由更上层根据所有权决定是否释放。
    Ok(frame)
}
\`\`\`

\`unmap\` 只移除映射并返回原物理页帧；它不会自动释放数据页帧，也不会自动释放所有变空的中间页表页。拿到返回值后，只有确认该帧不再被其他虚拟地址或进程共享，才可以调用页帧分配器的 unsafe 回收接口。

### 这段代码中的 unsafe 代表什么

\`map_to\` 是 unsafe API，因为错误映射可能让两个逻辑对象别名到同一块可变物理内存，或让正在使用的虚拟页突然指向另一块物理内存。它不是“调用了 unsafe 就安全”，而是要求调用者先证明映射关系不会破坏内存安全。

### 建议的调试输出

映射成功后记录：

- 虚拟页起始地址；
- 物理页帧起始地址；
- PageTableFlags；
- flush 前后的访问结果。

映射失败时区分 \`PageAlreadyMapped\` 和 \`FrameAllocationFailed\` 等错误。不要把所有错误都归类成“内存不足”。
