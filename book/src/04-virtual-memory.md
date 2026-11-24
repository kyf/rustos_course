# 04. 虚拟内存：地址为什么需要转换

## 虚拟地址与物理地址

物理地址描述 RAM 或设备在物理地址空间中的位置。虚拟地址是当前执行上下文使用的地址。在启用分页的系统中，CPU 发出的虚拟地址由 MMU（Memory Management Unit）根据当前页表转换为物理地址。

```text
CPU 产生 VA -> MMU 查询页表 -> 得到 PA -> 访问物理内存
```

## 为什么需要虚拟内存

- **隔离**：不同进程可以拥有互不相同的虚拟地址空间。
- **重定位**：程序不必知道自己最终被放到哪块物理 RAM。
- **权限控制**：页表项可以控制页面是否可读、可写、可执行，以及是否允许用户态访问。
- **稀疏映射**：不必为整个虚拟地址范围都分配物理内存。
- **共享**：多个进程可以把同一物理页映射到各自的虚拟地址。

## 相同 VA 为什么可以映射到不同 PA

```text
Address Space A: 0x4000_0000 -> Physical Frame A
Address Space B: 0x4000_0000 -> Physical Frame B
```

CPU 运行 A 时使用 A 的页表；运行 B 时使用 B 的页表。因此同一个虚拟地址可以翻译到不同物理位置。这是进程隔离的基础之一。

## Rust 指针是什么地址

在启用分页的内核中，Rust 指针表示当前地址空间中的虚拟地址，而不是天然的物理地址。因此，不能把物理页帧地址直接强转成指针并假设它可访问。必须先确认该物理地址已映射到当前虚拟地址空间，再使用对应虚拟地址访问。

如果内核建立了物理内存直接映射，常见转换形式是：

```text
virtual_address = physical_memory_offset + physical_address
```

这个公式只在对应物理地址确实被直接映射、偏移定义正确且没有溢出的情况下成立。它不是所有系统通用的地址转换公式。

## 映射存在不等于权限允许

一个地址可能已经映射，但因为当前权限不足而触发 Page Fault。用户态代码只能访问页表允许 User 的页面；内核态代码通常可以访问 Supervisor 页面。

## 思考题

1. 为什么两个进程可以使用相同的虚拟地址而不互相覆盖？
2. 为什么物理页帧地址不能直接作为 Rust 指针使用？
3. 一个页面已经映射，是否就一定可以读写？
4. 物理内存直接映射解决了什么问题？

## 本课结论

虚拟内存不是简单地给物理地址加一个偏移。它是一套由页表、MMU 和权限位共同实现的地址抽象与保护机制。


## 代码演示：把物理地址转换为直接映射虚拟地址

**来源标记：按历史课程中的物理内存偏移讨论重构。** 下面的函数只适用于 Bootloader 已经建立物理内存直接映射的地址范围。

```rust
use x86_64::{PhysAddr, VirtAddr};

fn phys_to_virt(
    physical_memory_offset: VirtAddr,
    physical_address: PhysAddr,
) -> Option<VirtAddr> {
    let raw = physical_memory_offset
        .as_u64()
        .checked_add(physical_address.as_u64())?;

    VirtAddr::try_new(raw).ok()
}
```

### 这段代码做了什么

- `PhysAddr` 和 `VirtAddr` 是不同类型，避免把两种地址不加区分地当成普通整数。
- `checked_add` 检查加法溢出。
- `VirtAddr::try_new` 检查结果是否符合当前 x86_64 48-bit canonical address 规则。
- 返回 `Option<VirtAddr>`，因为计算失败时不应该继续伪造一个有效地址。

如果确认地址有效并且映射存在，才可以在 unsafe 区域通过指针访问：

```rust
unsafe fn read_u64_through_direct_map(
    physical_memory_offset: VirtAddr,
    physical_address: PhysAddr,
) -> Option<u64> {
    let virtual_address = phys_to_virt(physical_memory_offset, physical_address)?;
    let pointer = virtual_address.as_ptr::<u64>();

    // SAFETY: 调用者必须保证该物理页帧属于可访问内存，
    // 已由直接映射覆盖，地址满足 u64 对齐，并且读取不会与可变访问冲突。
    Some(unsafe { pointer.read_volatile() })
}
```

这里最重要的不是 `as_ptr`，而是调用者承担的安全契约：目标物理页帧必须真的被映射、访问权限必须允许、不能与另一个可变引用形成别名冲突。

### 两个不能混淆的操作

```text
分配物理页帧：
    FreeListAllocator -> 返回一个 PhysFrame
    不会因此修改页表

通过直接映射访问：
    VA = physical_memory_offset + PA
    使用 Bootloader 已经建立的映射访问该页帧

映射到一个新的任意虚拟地址：
    Mapper::map_to(...)
    修改页表，并刷新对应 TLB 项
```

所以，分配器返回 Frame 和 Mapper 建立 VA → PA 是两个独立步骤。
