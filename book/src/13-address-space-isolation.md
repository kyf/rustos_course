# 13. 创建独立进程地址空间

## 本课目标

创建不同的页表根，让两个进程可以使用相同虚拟地址却访问不同物理页帧；同时理解为什么每个进程仍然需要可用的内核映射。

## 每个进程有自己的页表根

在常见 x86_64 四级分页中，CR3 指向当前使用的 PML4 根页表：

```text
Process A: CR3 -> PML4_A
Process B: CR3 -> PML4_B
```

两个 PML4 必须是不同的页表根，才能维护独立的用户空间映射。

## 相同 VA 映射到不同 PA

```text
Address Space A:
VA 0x4000_0000 -> PA 0x2000_0000

Address Space B:
VA 0x4000_0000 -> PA 0x3000_0000
```

CPU 当前使用哪张页表，决定了 `0x4000_0000` 被翻译到哪里。这是进程地址空间隔离最直观的验证方式。

## 为什么不能创建一张完全空白的新页表就切换

如果新 PML4 只包含用户空间映射，没有内核当前执行所需的映射，那么写入 CR3 后，CPU 可能无法继续取指或访问当前栈。

切换前必须保证新页表中有当前内核依赖的映射，例如内核代码与只读数据、内核可写数据和全局变量、当前内核栈、物理内存直接映射（如果内核依赖它）、中断处理程序和描述符等。

## 用户空间独立、内核空间共享

一种常见设计是：每个进程拥有自己的 PML4；用户空间页表结构独立；内核映射在各进程地址空间中保持一致；必要时共享内核下级页表结构，避免重复建立相同映射。

在常见 48 位四级分页、高半区内核布局中，PML4 索引 256–511 对应高半区；但这不是无条件适用于所有内核的复制规则。必须检查实际链接地址、页表布局、Bootloader 映射和内核使用的虚拟地址。不要未经检查就盲目复制整个高半区。

复制顶层 Entry 时，通常只是让新旧 PML4 指向同一套下级页表，并没有复制 PDPT、PD 或 PT 的内容。因此这是共享页表结构，不是两套完全独立的内核映射。

## 共享映射的权限

内核映射必须是 Supervisor-only，不能让用户态访问。用户页面则需要在页表路径上设置合适的 User 权限。

权限检查涉及页表遍历路径上的各级表项。仅仅把最后一级 PTE 标成 User，并不能绕过上级页表项的权限限制。

## 创建 AddressSpace 的职责

一个最小的地址空间对象可以持有自己的 PML4 物理页帧：

```rust
pub struct AddressSpace {
    pml4_frame: PhysFrame<Size4KiB>,
}
```

这只是第一版设计示意。实际实现还需要创建并清零 PML4 页帧、建立必要的内核映射、通过 Mapper 建立用户映射、记录用户页帧和下级页表的所有权，并在销毁时只释放本地址空间独占的资源。

不要把整个 `OffsetPageTable` 简单地以随意的 `'static mut` 引用长期保存。Mapper 对根页表有访问借用，必须认真处理生命周期和可变别名问题。

## 销毁地址空间时不能释放所有页表

如果多个进程共享内核下级页表，那么销毁进程 A 时不能递归释放所有可达页表页，否则进程 B 或内核本身可能继续引用已释放的页表。

销毁需要区分进程独占的用户页表页、进程独占的数据页帧、多个进程共享的物理页帧，以及所有进程共享的内核页表结构。共享资源需要引用计数、全局所有权或其他明确的生命周期管理策略。

## 实验：相同虚拟地址，不同物理页帧

### 实验 A：创建两个独立页表根

1. 读取当前 CR3，获得当前 PML4 的物理页帧。
2. 通过物理内存直接映射找到当前 PML4 的虚拟地址。
3. 申请新的物理页帧作为新 PML4。
4. 将新 PML4 清零。
5. 按内核实际内存布局，复制或共享必要的内核映射。
6. 定义 `AddressSpace`，持有自己的 PML4 物理页帧。

### 实验 B：验证 VA → PA 隔离

1. 创建 AddressSpace A 和 B。
2. 申请两个不同的数据物理页帧。
3. 在 A 和 B 中都映射虚拟地址 `0x4000_0000`。
4. A 映射到 Frame A，B 映射到 Frame B。
5. 切换到 A，在该 VA 写入 `100`。
6. 切换到 B，在该 VA 写入 `200`。
7. 分别切回 A、B，确认读到的值不同。

预期：

```text
Process A @ 0x4000_0000 -> 100
Process B @ 0x4000_0000 -> 200
```

注意：这是 Ring 0 中验证地址空间机制的实验，不等于已经实现 Ring 3。切换 CR3 前必须保证当前内核代码、栈和数据在新地址空间中仍可访问；还要考虑页表项权限、TLB 刷新和页帧所有权。多核环境还需要处理其他 CPU 核心的 TLB Shootdown。在完成可靠的切换与恢复路径前，不要把 CR3 切换实验放进普通内核流程中。

## 思考题

1. A 和 B 的 PML4 不同，但某个顶层 Entry 指向同一个 PDPT。如果修改这个 PDPT 下的 PTE，A 和 B 是否都会受到影响？为什么？
2. 新页表没有映射当前内核栈，切换 CR3 后可能发生什么？
3. A 和 B 共享内核页表结构，销毁 A 时能否递归释放所有页表？
4. 为什么相同 VA 映射不同 PA 能实现进程隔离？

## 本课结论

独立地址空间的关键不是“每个进程有一份不同的页表内存”这么简单，而是为每个进程维护独立的用户映射，同时保证必要的内核映射在切换前后始终有效，并正确管理共享资源的权限与生命周期。


## 代码演示：创建一个新的 PML4 根，并共享内核映射

**来源标记：历史第 13 课的目标和步骤已确认；下面代码是根据该方案重建的教学骨架，不是声称找回了历史逐字代码。** 在实际内核中，必须先核对内核链接地址、Bootloader 页表布局和内核映射范围。

```rust
use x86_64::{
    PhysAddr,
    VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator,
        PageTable,
        PhysFrame,
        Size4KiB,
    },
};

pub struct AddressSpace {
    pub pml4_frame: PhysFrame<Size4KiB>,
}

/// 创建新的 PML4，并共享当前地址空间中指定范围的内核顶层 Entry。
///
/// # Safety
/// - physical_memory_offset 必须是正确的物理内存直接映射偏移；
/// - allocator 返回的页帧必须是唯一且空闲的；
/// - kernel_p4_start 必须由真实内核虚拟内存布局确定；
/// - 共享的内核页表树在所有 AddressSpace 生命周期内必须保持有效。
pub unsafe fn create_address_space<A>(
    physical_memory_offset: VirtAddr,
    allocator: &mut A,
    kernel_p4_start: usize,
) -> Option<AddressSpace>
where
    A: FrameAllocator<Size4KiB>,
{
    assert!(kernel_p4_start < 512);

    // 1. 读取当前 CPU 的页表根。
    let (current_p4_frame, _) = Cr3::read();

    // 2. 通过直接映射访问当前 PML4。
    let current_p4_address =
        physical_memory_offset + current_p4_frame.start_address().as_u64();
    let current_p4 =
        unsafe { &*current_p4_address.as_ptr::<PageTable>() };

    // 3. 分配一个物理页帧作为新 PML4。
    let new_p4_frame = allocator.allocate_frame()?;
    let new_p4_address =
        physical_memory_offset + new_p4_frame.start_address().as_u64();
    let new_p4 =
        unsafe { &mut *new_p4_address.as_mut_ptr::<PageTable>() };

    // 4. 清空新页表，防止把未初始化内存当作 Entry。
    new_p4.zero();

    // 5. 只复制明确属于内核映射的 PML4 Entry。
    //    这里复制的是下级页表指针，不是递归复制整棵页表树。
    for index in kernel_p4_start..512 {
        new_p4[index] = current_p4[index].clone();
    }

    Some(AddressSpace {
        pml4_frame: new_p4_frame,
    })
}
```

### 这段代码真正做了什么

1. 从 CR3 读取当前 PML4 物理页帧。
2. 使用物理内存直接映射偏移，得到当前 PML4 的虚拟地址。
3. 从页帧分配器取得一个新的 4 KiB 页帧。
4. 将新页帧初始化为全空的 PML4。
5. 把指定的内核 PML4 Entry 复制到新 PML4 中。
6. 新 PML4 现在有独立的用户空间顶层表项，但内核区域仍指向原来的下级页表结构。

**复制一个 PML4 Entry 不等于复制整棵页表。** 如果 A 和 B 的顶层 Entry 指向同一个 PDPT，那么后续修改该 PDPT 下的 PTE，A 和 B 都会看到变化。这是共享页表结构的结果。

### 为什么不能把 256 写死后就认为所有内核都适用

在常见 48 位四级分页中，索引 256–511 是高半区，但具体内核映射布局由链接地址、Bootloader 配置和内核自身决定。应当先遍历当前 PML4，确认哪些 Entry 包含内核代码、内核栈、全局数据、物理内存直接映射和中断相关资源，再确定 `kernel_p4_start` 或更精确的复制范围。

### 代码演示：切换 CR3 的最小形式

```rust
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{PhysFrame, Size4KiB};

/// # Safety
/// 新 PML4 必须映射当前指令、当前内核栈、全局数据和必要的设备/中断区域。
pub unsafe fn switch_address_space(
    new_root: PhysFrame<Size4KiB>,
    flags: Cr3Flags,
) {
    unsafe { Cr3::write(new_root, flags); }
}
```

切换 CR3 不是普通的“切换一个变量”。下一条指令取指、当前 RSP 指向的栈访问、全局数据读取都会立即使用新页表。只要新页表缺少其中任何一个必要映射，CPU 就可能马上触发异常。

### 历史课程中的隔离验证目标

```text
Address Space A:
    0x4000_0000 -> Frame A

Address Space B:
    0x4000_0000 -> Frame B

A 中写入 100
B 中写入 200

切回 A，读取 100
切回 B，读取 200
```

这个实验验证的是 Ring 0 中两套页表根的映射隔离，不等于已经完成 Ring 3 用户程序运行。真实执行前还必须确保：

- 两个数据帧分别由分配器独占分配；
- 0x4000_0000 在两个地址空间中都映射且权限正确；
- 当前内核代码、栈和直接映射在两个页表中都可用；
- 保存旧 CR3 并在测试后恢复；
- 多核环境下处理其他 CPU 的 TLB Shootdown。

当前课程把它作为地址空间原理实验设计，不将它标记为已经在 QEMU 上完成验证。
