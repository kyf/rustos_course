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


## 代码演示：侵入式 Free List 页帧分配器

**来源标记：按历史第 10 课实现方案重构，不声称是聊天记录中的逐字原代码。** 历史课程明确讨论过：空闲页帧前 8 字节保存下一个空闲页帧的物理地址；分配器实例自身保存 free_head、physical_memory_offset 和 free_frames；实现 FrameAllocator 与 FrameDeallocator。

下面的实现把这一设计展开成代码。它依赖正确的物理内存直接映射，因此不能在普通用户态程序中运行。

~~~rust
use core::ptr::{read_volatile, write_volatile};

use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{
    FrameAllocator,
    FrameDeallocator,
    PhysFrame,
    Size4KiB,
};

const PAGE_SIZE: u64 = 4096;

pub struct FreeListFrameAllocator {
    // 分配器自己的元数据，不放在空闲页帧中。
    free_head: Option<PhysAddr>,
    physical_memory_offset: VirtAddr,
    free_frames: usize,
}

impl FreeListFrameAllocator {
    /// # Safety
    /// offset 必须指向覆盖可分配物理内存的直接映射区域。
    pub const unsafe fn new(physical_memory_offset: VirtAddr) -> Self {
        Self {
            free_head: None,
            physical_memory_offset,
            free_frames: 0,
        }
    }

    fn link_pointer(&self, frame: PhysFrame<Size4KiB>) -> *mut u64 {
        let virtual_address =
            self.physical_memory_offset + frame.start_address().as_u64();
        virtual_address.as_mut_ptr::<u64>()
    }

    pub fn free_frames(&self) -> usize {
        self.free_frames
    }

    /// 把 BootInfo 中可用区域切分为 4 KiB 页帧并加入 Free List。
    ///
    /// # Safety
    /// - regions 必须是 Bootloader 提供的真实内存地图；
    /// - 只能传入尚未被内核、Bootloader 或设备占用的 Usable 区域；
    /// - 直接映射必须覆盖这些页帧；
    /// - 不能对同一批区域重复初始化。
    pub unsafe fn add_usable_regions(&mut self, regions: &[MemoryRegion]) {
        for region in regions {
            if region.kind != MemoryRegionKind::Usable {
                continue;
            }

            let Some(start) = region.start.checked_add(PAGE_SIZE - 1) else {
                continue;
            };
            let start = start & !(PAGE_SIZE - 1);
            let end = region.end & !(PAGE_SIZE - 1);

            let mut address = start;
            while address < end {
                // 物理地址 0 保留作链表 next 指针的空值哨兵。
                if address != 0 {
                    let frame = PhysFrame::from_start_address(
                        PhysAddr::new(address)
                    )
                    .expect("aligned usable region");

                    // SAFETY: add_usable_regions 的调用者保证该页帧空闲且可直接映射。
                    unsafe { self.deallocate_frame(frame); }
                }

                address += PAGE_SIZE;
            }
        }
    }
}

// SAFETY: allocator 的唯一性由单一 free_head 链表维护；
// 每次分配只弹出链表头，每个页帧只会被返回一次。
unsafe impl FrameAllocator<Size4KiB> for FreeListFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let head = self.free_head?;
        let frame = PhysFrame::from_start_address(head).ok()?;

        // 空闲页帧的前 8 字节存放 next 的物理地址。
        let next = unsafe { read_volatile(self.link_pointer(frame)) };

        self.free_head = if next == 0 {
            None
        } else {
            Some(PhysAddr::new(next))
        };

        self.free_frames -= 1;
        Some(frame)
    }
}

impl FrameDeallocator<Size4KiB> for FreeListFrameAllocator {
    /// # Safety
    /// frame 必须确实空闲、属于该分配器管理的区域、未被重复释放，
    /// 并且不能仍被任何地址空间或页表使用。
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let old_head = self.free_head
            .map(|address| address.as_u64())
            .unwrap_or(0);

        // 把旧链表头写入当前空闲页帧的前 8 字节。
        unsafe { write_volatile(self.link_pointer(frame), old_head); }

        self.free_head = Some(frame.start_address());
        self.free_frames += 1;
    }
}
~~~

### 逐段讲解

**1. 分配器实例里保存什么？**

free_head 保存链表头的物理地址；physical_memory_offset 用于把物理页帧转换成可访问的虚拟地址；free_frames 只是统计数量。这些字段属于分配器对象本身。若分配器对象在 kernel_main 的局部变量中，它们就位于当前内核栈或由编译器安排的存储位置。

**2. 空闲页帧里保存什么？**

每个空闲页帧的起始 8 字节保存下一个空闲页帧的物理地址。这里不需要为链表节点另行分配内存，因此称为 intrusive（侵入式）链表。

**3. 为什么要做物理地址到虚拟地址的转换？**

PhysFrame 表示物理内存页帧，CPU 当前执行的 Rust 代码不能把物理地址直接当作普通指针解引用。代码先计算 physical_memory_offset + frame.start_address()，再得到可以访问的虚拟指针。

**4. 为什么这里使用 read_volatile / write_volatile？**

它们让对页帧内链表字段的读写以显式内存访问形式发生。它们不负责建立映射，也不提供多核同步；并发分配仍需要锁或其他同步机制。

**5. 为什么 deallocate_frame 是 unsafe？**

调用者必须保证没有重复释放、没有仍然有效的映射、没有其他进程共享使用该页帧。否则同一个物理页帧可能同时被分配给两个调用者，最终产生内存破坏。

### 从 BootInfo 初始化分配器

~~~rust
use bootloader_api::info::Optional;
use x86_64::VirtAddr;

fn physical_memory_offset(
    boot_info: &bootloader_api::BootInfo,
) -> Option<VirtAddr> {
    match boot_info.physical_memory_offset {
        Optional::Some(offset) => VirtAddr::try_new(offset).ok(),
        Optional::None => None,
    }
}
~~~

拿到偏移后，创建分配器并遍历 boot_info.memory_regions。MemoryRegions 实现了切片式的 Deref，因此可以调用 iter()。只有 MemoryRegionKind::Usable 才能进入分配器；Bootloader、UnknownBios、UnknownUefi 不能被当作空闲 RAM。

### 需要特别记住的边界

- 页帧分配器不负责虚拟地址分配。
- 页帧分配器不负责建立页表映射。
- unmap 后不代表一定可以释放该帧；必须确认它没有其他映射。
- 页表自身使用的页帧也来自物理页帧分配器，但销毁地址空间时必须分清共享页表和独占页表。
- 上面的侵入式链表是单核教学实现。多核使用前必须增加同步和所有权管理。

完整的可编译内存管理模块位于 [labs/kernel-memory](https://github.com/kyf/rustos_course/tree/docs/course-code-examples/labs/kernel-memory)。
