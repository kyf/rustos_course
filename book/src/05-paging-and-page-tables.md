# 05. x86_64 分页与多级页表

## 四级分页结构

在常见的 x86_64 四级分页模式下，虚拟地址被拆分为：

```text
| PML4 index | PDPT index | PD index | PT index | page offset |
|   9 bits   |   9 bits   |  9 bits  |  9 bits  |   12 bits   |
```

每级表有 512 个 8 字节表项，因此每张表占 4096 字节，也就是 4 KiB。

```text
CR3 -> PML4 -> PDPT -> PD -> PT -> 物理页帧 + 页内偏移
```

并非每个映射都一定需要完整的四级结构，例如大页映射可以在更高层结束。

## PML4 的作用

PML4 是四级分页的根页表。它有 512 个表项，每个表项通常指向下一级页表，或者在特定配置下指向更大页面。PML4 本身占用一个 4 KiB 页帧。

## CR3 保存什么

CR3 保存当前地址转换所使用的页表根的物理地址（还可能包含与 PCID 等功能有关的字段，取决于 CPU 配置）。

- CR3 中的根地址是物理地址。
- 内核要通过 Rust 指针访问 PML4，需要找到该物理页帧对应的虚拟映射。
- 写入 CR3 会改变 CPU 使用的地址空间根。

不要把 CR3 当成普通虚拟指针。

## 为什么采用多级页表，而不是一张巨大的稠密表

四级页表的关键优势是**按需分配下级表**。一个 PML4 表本身只有 4 KiB；某个 PML4 Entry 未使用时，只是一个空表项，并不会自动分配它下面的 PDPT、PD 和 PT。只有虚拟地址落入相应范围、需要建立映射时，内核才分配所需的下级表。

每个 PML4 Entry 覆盖 512 GiB 虚拟范围，但这不表示系统会为每个 Entry 预留 512 GiB RAM。一个 4 KiB PT 可以描述 2 MiB 的 4 KiB 页面映射；如果 512 GiB 范围全部用 4 KiB 页面密集映射，光 PT 层大约需要 1 GiB 页表内存，另外还有更高层页表；它仍远小于 512 GiB。若只映射少数分散页面，多级页表只需创建这些页面所在分支的下级表。

## Page、PhysFrame 与 PageTableFlags

`Page` 是内核代码对一段页对齐虚拟地址范围的抽象，`PhysFrame` 是对一段页对齐物理地址范围的抽象。硬件页表项在页粒度上记录“虚拟 Page 的基址映射到物理 Frame 的基址”，页内偏移保持不变。使用 `Page` / `PhysFrame` 类型可以让 API 表达对齐和页粒度约束，减少把任意字节地址误当成页基址的错误。

`PageTableFlags` 表示页表项的属性，常见标志包括：

- `PRESENT`：该映射存在。
- `WRITABLE`：允许写入（仍需结合各级表项权限判断）。
- `USER_ACCESSIBLE`：允许用户态访问（权限由遍历路径共同决定）。
- `NO_EXECUTE`：禁止从该页执行指令（需 CPU 支持并启用相应机制）。
- `WRITE_THROUGH`、`NO_CACHE`：影响缓存策略。
- `ACCESSED`、`DIRTY`：由硬件维护的访问/写入状态位。
- `HUGE_PAGE`：在支持的层级使用大页映射。
- `GLOBAL`：在符合架构条件时，允许该转换跨部分地址空间切换保留。

## Bootloader 与页表

Bootloader 通常会建立一套足以运行内核的页表。内核接管后，可以继续使用它，也可以建立自己的页表。

如果内核要更换页表，必须保证切换瞬间仍然映射当前正在执行的内核代码、内核栈、全局数据、物理内存直接映射（如果内核依赖它），以及中断处理需要的代码、栈和描述符等资源。否则切换 CR3 后，CPU 可能立即发生 Page Fault。

## BootInfo 与页表不是一回事

BootInfo 是描述启动环境的数据；PageTable 是 MMU 用来翻译地址的数据结构。BootInfo 可能告诉内核物理内存偏移、内存区域等信息，但它不是页表本身。

## OffsetPageTable 与 PageTable

以 `x86_64` crate 的抽象为例：

- `PageTable` 表示一张页表的数据结构。
- `OffsetPageTable` 是一种 Mapper 实现，它利用物理内存直接映射偏移，帮助内核访问和遍历页表。

OffsetPageTable 不代表另一种硬件页表格式；它是内核侧操作页表的工具抽象。

## 物理地址直接映射

如果内核把物理内存某个范围映射到连续的虚拟区域，那么在该映射范围内可以使用：

```text
virtual = physical_memory_offset + physical
```

需要确认该物理地址处于直接映射覆盖范围、计算没有溢出、目标虚拟地址有效且页表权限允许访问。

## 思考题

1. 为什么 CR3 保存的是物理根地址，而不是 Rust 指针？
2. PML4、PDPT、PD、PT 分别解决什么问题？
3. BootInfo 是否就是 Bootloader 创建的页表？
4. 如果新页表没有映射当前内核栈，切换 CR3 后可能发生什么？

## 本课结论

页表是硬件定义的数据结构；`OffsetPageTable` 是内核访问页表的抽象；CR3 告诉 CPU 当前使用哪张根页表。


## PML4 索引为什么左移 39 位

四级分页的虚拟地址中，PML4 index 占 bit 47..39，共 9 bit。每个 PML4 Entry 覆盖的虚拟地址范围是：

\[
2^{9+9+9+9+12}=2^{48}\text{ bytes}
\]

其中单个 PML4 Entry 覆盖：

\[
2^{9+9+9+12}=2^{39}\text{ bytes}=512\text{ GiB}
\]

因此，如果选中 PML4 index 为 5，它对应的 512 GiB 虚拟地址区域起点可以由下式得到：

\[
5 \ll 39 = 0x0000\_0028\_0000\_0000
\]

这行代码：

```rust
let target_virtual_addr = VirtAddr::new((free_p4_index as u64) << 39);
```

是在构造该 PML4 槽位所覆盖虚拟区域的起始地址。它**不是**从 PML4 数组中取出下级页表地址，也不是把页表项里的物理地址转换成虚拟地址。

需要区分两件事：

- PML4 的一个已使用 Entry 中，存放的是下一级页表（通常是 PDPT）的物理地址和标志。
- 一个空闲 PML4 Entry 没有下级表指针。将它的索引左移 39 位，是为了构造落在该槽位覆盖范围内的虚拟地址。

对于高半区索引，还必须按 48 位 canonical address 规则进行符号扩展；不能把简单左移结果无条件当成合法的高半区虚拟地址。

## 如何观察 Bootloader 当前建立的页表

一个常见的检查思路是：

1. 在当前逻辑 CPU 上读取 CR3，得到当前 PML4 根页帧的物理地址。
2. 确认 Bootloader 建立的物理内存直接映射覆盖这个页帧。
3. 使用 `physical_memory_offset + pml4_physical_address` 得到可访问的虚拟地址。
4. 在 `unsafe` 代码中把该虚拟地址视为 `PageTable`，读取其 Entry。
5. 继续沿 Present Entry 指向的下级表遍历，打印各级索引、物理地址和 Flags。

这是检查当前 CPU 正在使用的页表，不是读取某个抽象的“BootInfo 页表”。如果内核已经切换过 CR3，读到的就是切换后的根页表。直接解引用页表内存涉及裸指针和别名安全，必须保证地址映射正确且访问期间没有并发修改。

以 `x86_64` crate 为例，`PageTable` 是页表内存本身的结构化视图；`OffsetPageTable` 则是使用物理内存偏移来访问和修改页表的 Mapper 抽象。前者不是后者的“底层页表版本”——二者处于不同抽象层。


## 代码演示：读取 CR3 并取得当前 PML4

**来源标记：按历史课堂中“查看 Bootloader 建立的页表”的讨论重构。** 使用下面的代码之前，必须已经启用物理内存直接映射，并确认偏移值正确。

```rust
use x86_64::{
    registers::control::Cr3,
    structures::paging::{OffsetPageTable, PageTable},
    VirtAddr,
};

/// 读取当前 CPU 正在使用的 PML4。
///
/// # Safety
/// - physical_memory_offset 必须对应完整、正确的物理内存直接映射；
/// - 当前 CR3 必须指向有效的四级页表；
/// - 调用者必须保证不会同时创建另一个可变 PageTable 引用。
unsafe fn active_level_4_table(
    physical_memory_offset: VirtAddr,
) -> &'static mut PageTable {
    let (level_4_frame, _flags) = Cr3::read();
    let physical_address = level_4_frame.start_address();
    let virtual_address =
        physical_memory_offset + physical_address.as_u64();

    // SAFETY: 由函数调用者保证偏移、页表根地址和独占访问条件。
    unsafe { &mut *virtual_address.as_mut_ptr::<PageTable>() }
}

/// 创建一个使用物理内存直接映射偏移的 Mapper。
///
/// # Safety
/// 只能在当前页表根和物理内存映射满足 active_level_4_table 的契约时调用。
unsafe fn init_offset_page_table(
    physical_memory_offset: VirtAddr,
) -> OffsetPageTable<'static> {
    let level_4_table =
        unsafe { active_level_4_table(physical_memory_offset) };

    // SAFETY: level_4_table 是当前有效 PML4，offset 是正确的直接映射偏移。
    unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}
```

### 逐步理解

1. `Cr3::read()` 返回当前 PML4 的物理页帧和 CR3 标志位。CR3 中存放的是物理页表根，不是 Rust 虚拟指针。
2. `start_address()` 取得 PML4 页帧的物理起始地址。
3. `physical_memory_offset + physical_address` 通过 Bootloader 建立的直接映射找到同一块页表内存的虚拟地址。
4. `as_mut_ptr::<PageTable>()` 只是把数值解释为指针，并不负责创建映射。
5. `OffsetPageTable::new` 创建的是内核侧 Mapper 抽象；硬件页表格式没有因为创建这个 Rust 对象而改变。

`'static mut` 在这里是教学内核常见的简化方式，但它要求非常严格：不能反复调用函数制造多个指向同一 PML4 的可变引用。正式内核应由内存管理器统一持有 Mapper，并通过受控借用暴露操作接口。

## 代码演示：从虚拟地址拆出四级页表索引

```rust
use x86_64::VirtAddr;

fn dump_page_table_indexes(raw: u64) {
    let address = VirtAddr::try_new(raw).expect("non-canonical address");

    let p4 = u16::from(address.p4_index());
    let p3 = u16::from(address.p3_index());
    let p2 = u16::from(address.p2_index());
    let p1 = u16::from(address.p1_index());
    let offset = u16::from(address.page_offset());

    // 在内核中把这些值输出到串口。
    let _ = (p4, p3, p2, p1, offset);
}
```

以 4 KiB 页面为例，低 12 bit 是页内偏移；之后每 9 bit 分别是 PT、PD、PDPT、PML4 索引。索引范围都是 0 到 511。这个拆分解释了为什么页表每一级有 512 个 Entry。

## 代码演示：PML4 Entry 对应的 512 GiB 区域起点

```rust
use x86_64::VirtAddr;

fn pml4_slot_start(index: u16) -> Option<VirtAddr> {
    if index >= 512 {
        return None;
    }

    let raw = (index as u64) << 39;
    VirtAddr::try_new(raw).ok()
}
```

例如 index 为 5 时，左移 39 位得到该 PML4 槽位覆盖区域的低半区起点。注意：高半区槽位需要正确符号扩展，不能把简单左移结果无条件当成 canonical 地址。
