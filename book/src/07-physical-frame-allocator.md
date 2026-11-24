# 07. 物理页帧分配器

## 分配器管理的是什么

物理页帧分配器管理可供内核使用的物理 4 KiB 页帧。它不负责决定虚拟地址放在哪里，也不直接完成 VA 到 PA 的映射。

| 组件 | 管理对象 | 主要职责 |
|---|---|---|
| Physical Frame Allocator | 物理页帧 | 分配/回收物理页 |
| Virtual Address Allocator | 虚拟地址范围 | 预留/释放 VA 区间 |
| Page Mapper | 页表映射 | 建立/撤销 VA → PA |
| Heap Allocator | 小对象内存块 | 为 `Box`、`Vec` 等分配对象内存 |

## 哪些物理内存可以分配

不能把整段 RAM 都当成空闲内存。Bootloader、内核镜像、页表、启动信息、帧缓冲区和设备区域等都可能需要保留。

BootInfo 的 MemoryRegion 通常会描述不同区域类型，例如：
- `Usable`：可以交给内核分配器管理的内存。
- `Bootloader`：Bootloader 正在使用或需要保留的内存。
- `UnknownBios`：固件提供但语义不够明确的区域，不能随意当成可用内存。
- 其他类型：按启动协议和内核实现判断。

必须遵循启动协议对每种区域的定义，不能仅凭地址落在 RAM 范围内就分配。

## Free List

Free List 是常见的页帧分配结构。空闲页帧通过链表连接：

```text
free_head -> frame A -> frame B -> frame C -> null
```

分配时取出链表头；释放时把页帧重新挂回链表。

intrusive Free List 会把链表节点信息直接放在空闲页帧内部。这样可以减少额外元数据分配，但要求该页帧确实空闲、节点地址已映射，且分配后不能再把它当作链表节点访问。

## 分配与回收契约

概念接口：

```rust
trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame>;
}

trait FrameDeallocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame);
}
```

这是职责示意，不保证与你项目中的 trait 签名完全一致。释放操作需要保证页帧确实由该分配器分配、当前没有仍在使用的映射、不会重复释放，也不会释放仍被其他地址空间共享的页帧。

## 初始化分配器

1. 读取 BootInfo 的内存区域。
2. 筛选可用区域。
3. 排除内核、Bootloader 和其他保留区域。
4. 按 4 KiB 对齐切分成页帧。
5. 将空闲页帧加入 Free List 或其他数据结构。
6. 保留必要的分配器元数据。

如果分配器通过直接映射访问页帧内容，还必须确认对应物理内存已经映射。

## 思考题

1. 为什么不能把 BootInfo 中所有 MemoryRegion 都放进 Free List？
2. intrusive Free List 为什么能减少额外元数据？
3. 为什么释放页帧需要比申请页帧更严格的安全约束？
4. 如果一个物理页帧仍映射在另一个进程中，当前进程退出时能否释放它？

## 本课结论

物理页帧分配器只负责物理页的所有权与生命周期。虚拟地址分配和页表映射是另外两项职责。
