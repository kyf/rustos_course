# 06. 页大小、对齐与物理页帧

## 页与页帧

在常见的 4 KiB 基础分页中：

- **Page（页）**：虚拟地址空间中的 4 KiB 对齐区域。
- **Frame（页帧）**：物理地址空间中的 4 KiB 对齐区域。

页表把一个虚拟 Page 映射到一个物理 Frame。4 KiB = 4096 bytes = `0x1000`，因此基础页帧的物理起始地址低 12 位必须为 0。

## 为什么需要对齐

页表项保存页帧基地址和标志位。硬件按页大小切分地址，所以页帧基地址必须满足对齐要求。对齐也简化页表索引、分配器管理、映射与解除映射。

## align_down 与 align_up

对于对齐边界 `A`（例如 `0x1000`）：

```text
align_down(x, A) = x 向下取最近的 A 的倍数
align_up(x, A)   = x 向上取最近的 A 的倍数
```

当 A 为 2 的幂时，常见位运算实现：

```rust
fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
```

真实代码还要检查 `align` 是否为 2 的幂，并处理加法溢出。

## 一个容易混淆的边界例子

考虑半开区间 `[start, end)`：

```text
start = 0x1003
end   = 0x5005
```

**A. 找出覆盖整个区间所需的所有页**

- 起点向下对齐：`0x1000`
- 终点向上对齐：`0x6000`
- 覆盖范围 `[0x1000, 0x6000)`，共 5 个 4 KiB 页帧。

**B. 找出完全包含在区间内部的完整页**

- 起点向上对齐：`0x2000`
- 终点向下对齐：`0x5000`
- 完整页范围 `[0x2000, 0x5000)`，共 3 个页帧。

两种算法都可能正确，取决于问题是在“覆盖”还是“完全包含”。不能只记住函数名而忽略边界语义。

## 为什么 4 KiB 是常见页大小

4 KiB 是 x86_64 常见基础页大小。更大的页面（如 2 MiB、1 GiB）也可能被支持，用于减少页表层级和 TLB 压力，但分配粒度更大，灵活性也会下降。

## 为什么保护第一个页

很多内核会故意不映射虚拟地址空间最开始的一页或一段区域。这样空指针附近的访问更容易触发 Page Fault，而不是意外访问有效物理位置。这是防御性设计，不是硬件强制要求。

## malloc(100) 一定需要新页帧吗

不一定。堆分配器通常先从已经映射的大块内存中切分小对象。只有当现有堆空间不足、需要向底层申请更多内存时，才可能需要新的页帧。

## 物理页帧地址不能直接当指针

物理地址 `0x0080_0000` 不自动等于当前可解引用的虚拟地址。必须确认它被映射到当前地址空间，并使用相应虚拟地址访问。

## 思考题

1. 为什么页帧起始地址必须对齐？
2. `[0x1003, 0x5005)` 覆盖多少个 4 KiB 页？完全包含多少个完整页？
3. `malloc(100)` 为什么可能不需要申请新页帧？
4. 为什么内核常常不映射第一个虚拟页？

## 本课结论

对齐计算必须结合区间语义。页帧是物理内存管理的基本单位，而 Page 是虚拟内存映射的基本单位；二者通过页表建立关系。


## 代码演示：安全地实现 align_up / align_down

**来源标记：按历史课程中的对齐练习重构，并补充溢出检查。**

\`\`\`rust
const PAGE_SIZE: u64 = 4096;

fn align_down(value: u64, alignment: u64) -> Option<u64> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return None;
    }

    Some(value & !(alignment - 1))
}

fn align_up(value: u64, alignment: u64) -> Option<u64> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return None;
    }

    let adjusted = value.checked_add(alignment - 1)?;
    Some(adjusted & !(alignment - 1))
}

fn covered_page_count(start: u64, end: u64) -> Option<u64> {
    if start >= end {
        return Some(0);
    }

    let first = align_down(start, PAGE_SIZE)?;
    let limit = align_up(end, PAGE_SIZE)?;
    Some((limit - first) / PAGE_SIZE)
}
\`\`\`

### 用课堂练习数据验证

区间采用半开区间 \`[start, end)\`：

\`\`\`text
start = 0x1003
end   = 0x5005

覆盖整个区间：
    align_down(start) = 0x1000
    align_up(end)     = 0x6000
    (0x6000 - 0x1000) / 0x1000 = 5 页

只计算完全位于原区间内部的整页：
    align_up(start)   = 0x2000
    align_down(end)   = 0x5000
    (0x5000 - 0x2000) / 0x1000 = 3 页
\`\`\`

之前练习中的 \`0x2000\` 和 \`0x5000\` 计算的是“完全包含在区间内部的整页”，不是“覆盖整个区间所需的页”。这两个问题使用不同的对齐方向。

### 用 x86_64 crate 表示页和页帧

\`\`\`rust
use x86_64::{
    PhysAddr,
    VirtAddr,
    structures::paging::{Page, PhysFrame, Size4KiB},
};

fn make_page_and_frame(
    virtual_address: u64,
    physical_address: u64,
) -> Result<(Page<Size4KiB>, PhysFrame<Size4KiB>), ()> {
    let virtual_address = VirtAddr::try_new(virtual_address).map_err(|_| ())?;
    let physical_address = PhysAddr::try_new(physical_address).map_err(|_| ())?;

    let page = Page::from_start_address(virtual_address).map_err(|_| ())?;
    let frame = PhysFrame::from_start_address(physical_address).map_err(|_| ())?;

    Ok((page, frame))
}
\`\`\`

这里使用 \`from_start_address\`，因此传入的地址必须已经按 4 KiB 对齐。若手里拿到的是任意字节地址，应先用 \`Page::containing_address\` 或显式对齐，而不是假装它已经是页起点。
