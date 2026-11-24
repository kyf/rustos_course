# 15. BootInfo 与物理内存地图

## BootInfo 是启动交接信息，不是页表

Bootloader 在把控制权交给内核时，通常会传递一份 BootInfo。它可能包含 MemoryRegion 列表、物理内存偏移、帧缓冲区、内核加载位置等信息。

需要区分：

- **BootInfo**：描述启动环境的数据结构。
- **Page Table**：CPU 的 MMU 用于虚拟地址转换的数据结构。
- **Physical Frame Allocator**：内核接管后管理可分配物理页帧的软件组件。

Bootloader 可以提前建立页表，但 BootInfo 本身不是那张页表。内核也不需要向 Bootloader 发送“我又分配了一个页帧”的通知；内核接管后自行维护分配器和页表。

## MemoryRegion 为什么有不同 kind

内存地图把物理地址空间划分为不同用途的区域。常见类别可能包括：

- `Usable`：按照启动协议可交给内核分配器使用的 RAM。
- `Bootloader`：Bootloader 正在使用或要求保留的区域。
- `UnknownBios`：固件/平台报告但当前启动协议无法明确归类的区域。
- 其他保留、设备或帧缓冲区类别：必须按协议定义处理。

内核不能只因为某段地址“看起来位于 RAM 范围”就分配它。最安全的第一版策略是只把协议明确标记为 `Usable` 的范围交给页帧分配器，再扣除内核自身、启动信息和内核已占用的页帧。

## 为什么 QEMU 设置 512 MiB，可用内存可能只有约 509 MiB

QEMU 的 `-m 512M` 表示虚拟机器配置的 RAM 容量，并不意味着内核启动时可以把每一个字节都拿来分配。内核镜像、Bootloader、启动栈、页表、BootInfo、帧缓冲区或其他保留用途都会占用或排除一部分区域。内存区域还必须按页对齐，不能把边界不完整的页帧交给分配器。

因此 `Usable` 总量比配置 RAM 少一些是正常现象。差额并不固定为 3 MiB；它取决于镜像大小、启动方案、QEMU 机器配置和 Bootloader 的内存布局。

## 为什么 UnknownBios 加起来可能超过 QEMU 的 RAM

MemoryRegion 描述的是物理地址地图，不是简单的“已安装 RAM 条形图”。固件可能报告高地址保留窗口、设备映射区或其他平台地址范围；这些地址可能远高于实际 RAM 容量。

例如某些区域位于 `0xFD00_0000_00` 附近，远超 512 MiB RAM 的地址范围。它们不能因此被当成真实可用 RAM，也不应加入页帧分配器。

分析内存地图时：

1. 先按 `kind` 分类，而不是把所有 size 相加。
2. 对 `Usable` 区域求和，作为启动协议允许内核管理的候选范围。
3. 对 `UnknownBios` 和设备/保留区域保持保守，不要分配。
4. 检查区域是否位于实际 RAM 地址范围内、是否有重叠，以及协议对地址范围的定义。

## 物理内存直接映射的含义

若 Bootloader/内核建立了物理内存直接映射，某个物理地址 `P` 在映射覆盖范围内可能对应虚拟地址：

```text
VA = physical_memory_offset + P
```

这里的 offset 是内核虚拟地址与物理地址之间的直接映射偏移。该公式成立的前提是目标物理地址确实在直接映射范围内，且计算没有溢出。

### 分配页帧会自动建立映射吗？

**页帧分配本身不会写页表。** 分配器只把一个物理页帧从“空闲”状态交给调用者。

如果 Bootloader 已经建立的直接映射覆盖该物理地址，那么对应的直接映射虚拟地址早已存在，内核可以通过 `offset + physical_address` 访问它；这不是分配器刚刚创建的新映射。

如果要把同一物理页帧映射到另一个任意虚拟地址，则需要通过 Mapper 修改页表，例如调用 `map_to()`，并按要求刷新 TLB。

`Mapping::Dynamic` 一类配置通常表示直接映射基址由 Bootloader 动态选择/确定；它不等于“每次分配页帧时自动创建一个新的 PTE”。具体映射覆盖范围和行为必须以项目锁定的 Bootloader 版本文档及实际页表为准。

## 虚拟地址空间大小不等于可用物理 RAM

虚拟地址空间是地址范围，物理页帧才是 RAM 资源。内核可以预留远大于物理 RAM 的虚拟地址范围，只有真正映射时才需要物理页帧和页表结构。没有映射的虚拟页不会自动占用对应大小的物理 RAM。

因此虚拟地址分配器不应该简单设置成 `Usable` 物理内存总量。它应管理合法、保留区之外的虚拟地址范围，并在实际映射时由物理页帧分配器提供后端资源。

## 本课结论

BootInfo 帮助内核理解启动环境；MemoryRegion 帮助内核判断哪些物理范围可以使用；直接映射让内核能按规则访问部分物理内存；物理页帧分配和虚拟映射仍然是不同职责。


## 代码演示：逐项读取 BootInfo 内存地图

**来源标记：新增补充示例，类型与字段对应 bootloader_api 0.11.17。**

\`\`\`rust
use bootloader_api::{
    info::{MemoryRegion, MemoryRegionKind, Optional},
    BootInfo,
};
use x86_64::VirtAddr;

fn region_name(region: &MemoryRegion) -> &'static str {
    match region.kind {
        MemoryRegionKind::Usable => "Usable",
        MemoryRegionKind::Bootloader => "Bootloader",
        MemoryRegionKind::UnknownBios(_) => "UnknownBios",
        MemoryRegionKind::UnknownUefi(_) => "UnknownUefi",
        _ => "Other",
    }
}

fn inspect_boot_info(boot_info: &BootInfo) {
    for region in boot_info.memory_regions.iter() {
        let start = region.start;
        let end = region.end; // end 是 exclusive
        let length = end.saturating_sub(start);
        let kind = region_name(region);

        // 在内核中把 start/end/length/kind 输出到串口。
        let _record = (start, end, length, kind);
    }
}

fn get_physical_memory_offset(boot_info: &BootInfo) -> Option<VirtAddr> {
    match boot_info.physical_memory_offset {
        Optional::Some(offset) => VirtAddr::try_new(offset).ok(),
        Optional::None => None,
    }
}
\`\`\`

### 为什么只把 Usable 放进分配器

Bootloader API 对内存地图的定义很明确：只有 \`Usable\` 区域可以自由交给内核使用；\`Bootloader\` 区域包含 Bootloader 使用的映射、页表和 BootInfo 等内容，不能当成空闲内存。Unknown 类型也不能仅因为它的起始地址位于 RAM 附近就被分配。

### 为什么所有 MemoryRegion 的 size 相加可能大于 QEMU -m

\`-m 512M\` 表示配置的 RAM 容量；MemoryRegion 还可能描述高地址固件保留区或设备映射区。它们是物理地址空间地图的一部分，不代表都是真实 RAM。应按 kind 分类，并只统计协议允许分配的 Usable 区域。

\`UnknownBios(u32)\` 的参数是固件 E820 类型值；\`UnknownUefi(u32)\` 的参数是 UEFI 内存类型值。不要把这个参数误认为内存大小。
