# 11. 虚拟地址分配器

## 为什么还需要虚拟地址分配器

物理页帧分配器回答：“给我一个可用的物理页帧。”

虚拟地址分配器回答：“给我一段未被占用、满足对齐要求的虚拟地址范围。”

Mapper 回答：“把这段虚拟地址映射到指定物理页帧。”

三者不能混为一谈。一个地址空间可以先预留 VA，之后按需映射物理页；也可以把同一个 PA 映射到多个 VA。

## Bitmap

Bitmap 用一个 bit 表示一个管理单元是否已占用：

```text
bit:   0 1 2 3 4 5 6 7
state: 0 1 1 0 0 1 0 0
```

如果每个 bit 表示一个 4 KiB 页，那么管理 1 GiB 虚拟范围需要：

`1 GiB / 4 KiB = 262144 bits = 32 KiB`

Bitmap 结构简单、元数据开销可预测，但查找连续空闲区间时可能需要扫描。

### Bitmap 元数据大小：bool 数组与 bitset

如果管理 4096 个页面：

- `[bool; 4096]` 通常每个元素占 1 byte，数组约占 4096 bytes（Rust 的 `bool` 大小为 1 byte）。
- 真正的 bitset 每页只用 1 bit，4096 bits = 512 bytes。

所以 bitset 比 bool 数组节省约 3584 bytes。若 bitmap 是局部变量，这些字节通常位于当前函数栈帧；若作为分配器字段，则取决于分配器本身放在哪里。不要仅凭“数组长度”推断整个分配器一定在栈上。

## First Fit

First Fit 从管理范围起点开始扫描，找到第一个满足长度和对齐要求的连续空闲区间。

优点是实现简单，找到第一个合适区间即可返回。缺点是可能产生外部碎片，长期分配/释放后扫描成本可能增加，结果也依赖扫描起点和分配历史。

可以进一步考虑 Next Fit、Best Fit、空闲区间树等策略，但学习初期先掌握 Bitmap + First Fit 即可。

## 规范虚拟地址

在常见的 48 位 x86_64 虚拟地址模式中，虚拟地址必须满足 canonical address 规则：高位需要正确符号扩展。并不是 64 位整数的每一种取值都能作为合法线性地址。

虚拟地址分配器必须管理合法的地址区间，并避开内核已占用区域、用户空间和内核空间边界、空指针保护页或保留区域、已映射或预留的区域。

如果 CPU 启用了 5 级分页（LA57），地址规范规则和地址空间范围会不同，不能把 48 位布局写死为所有 x86_64 配置。

## 页面粒度与 malloc 的粒度不是一回事

内核的虚拟地址分配器通常以页为单位预留地址范围，但用户程序调用 `malloc(100)` 并不意味着每次都要向内核申请一个 4 KiB 页面。用户态堆分配器会从已经获得的大块堆区域中切分出 100 bytes；当堆空间不足时，才向内核申请或映射更多页面。页粒度是虚拟内存管理的粒度，不是所有对象分配的最小粒度。

## 连续虚拟地址与不连续物理页帧

一个长度为 N 页的连续虚拟区间，可以逐页映射到 N 个不连续的物理页帧：

```text
连续 VA:  Page 0 -> Frame 7
          Page 1 -> Frame 2
          Page 2 -> Frame 19
```

程序看到的虚拟区间仍然连续。页表按页记录这些映射；物理页帧不连续不会自动让虚拟地址不连续。页表中间层的额外开销取决于虚拟地址分布落入多少不同的页表分支，而不是简单地按物理帧是否连续计算。

## 申请与释放

虚拟地址分配器通常只负责记录区间占用状态，不一定直接建立页表映射。释放虚拟范围前，调用者需要确保该范围属于当前分配器、已经处理其中的映射、不会让仍在使用的指针失效，也没有其他对象继续引用该范围。

推荐的完整释放顺序通常是：先撤销页表映射，再让相关 TLB 转换失效，然后释放本映射独占的物理页帧，最后释放虚拟地址范围。若物理页帧被其他映射共享，则不能因为当前 VA 被释放就释放该页帧。多核环境还必须等待其他 CPU 完成对应的 TLB shootdown，才能安全复用旧页帧。

## 思考题

1. 为什么有了物理页帧分配器，还需要虚拟地址分配器？
2. Bitmap 中一个 bit 可以代表什么？
3. First Fit 的优点和缺点是什么？
4. 为什么虚拟地址分配器必须考虑 canonical address？

## 本课结论

虚拟地址分配与物理页帧分配是两个独立问题。把“找 VA”“找 PA”“建立映射”拆开，才能构建职责清晰的内存管理系统。


## 非 canonical 地址会怎样

在启用 4 级分页、使用 48 位线性地址的常见 x86_64 配置中，合法地址的 bit 63..48 必须等于 bit 47 的符号扩展值。若地址不符合 canonical 规则，CPU 通常在地址转换之前就拒绝该地址；普通内存操作一般触发 `#GP`，涉及栈段的特定访问可能触发 `#SS`，而不是因为页表没有映射就触发普通 Page Fault。

若启用 LA57（5 级分页），canonical 地址规则改为基于 57 位线性地址。虚拟地址分配器必须依据实际 CPU 分页模式和内核布局选择合法范围。


## 代码演示：Bitmap + First Fit

**来源标记：按历史第 11 课实现思路重构。** 历史课程明确讨论过 Bitmap、First Fit、连续虚拟页分配、释放，以及 bool 数组和 bitset 的空间差异。下面给出一份可单独测试的实现；实际内核还要把返回的页索引转换为当前 AddressSpace 中的虚拟地址。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeError {
    InvalidRange,
    DoubleFree,
}

pub struct BitmapAllocator<'a> {
    words: &'a mut [u64],
    page_count: usize,
}

impl<'a> BitmapAllocator<'a> {
    /// words 必须至少有 ceil(page_count / 64) 个 u64。
    /// 构造时清零，表示所有受管理页面最初都是空闲的。
    pub fn new(words: &'a mut [u64], page_count: usize) -> Option<Self> {
        let required_words = page_count.checked_add(63)? / 64;
        if words.len() < required_words {
            return None;
        }

        words.fill(0);
        Some(Self { words, page_count })
    }

    fn is_used(&self, page: usize) -> bool {
        let word = page / 64;
        let bit = page % 64;
        (self.words[word] & (1u64 << bit)) != 0
    }

    fn set_used(&mut self, page: usize, used: bool) {
        let word = page / 64;
        let bit = page % 64;
        let mask = 1u64 << bit;

        if used {
            self.words[word] |= mask;
        } else {
            self.words[word] &= !mask;
        }
    }

    /// 从低页号开始查找第一个满足数量和对齐要求的连续区间。
    /// 返回起始页索引，而不是虚拟地址。
    pub fn allocate_first_fit(
        &mut self,
        count: usize,
        align_pages: usize,
    ) -> Option<usize> {
        if count == 0 || align_pages == 0 || count > self.page_count {
            return None;
        }

        let last_start = self.page_count - count;

        for start in 0..=last_start {
            if start % align_pages != 0 {
                continue;
            }

            let end = start + count;
            let range_is_free = (start..end).all(|page| !self.is_used(page));

            if range_is_free {
                for page in start..end {
                    self.set_used(page, true);
                }
                return Some(start);
            }
        }

        None
    }

    /// 标记一段预留区间为已占用，例如内核映像或保护页。
    pub fn reserve(&mut self, start: usize, count: usize) -> Result<(), FreeError> {
        let end = start.checked_add(count).ok_or(FreeError::InvalidRange)?;
        if count == 0 || end > self.page_count {
            return Err(FreeError::InvalidRange);
        }

        if (start..end).any(|page| self.is_used(page)) {
            return Err(FreeError::InvalidRange);
        }

        for page in start..end {
            self.set_used(page, true);
        }
        Ok(())
    }

    pub fn deallocate(
        &mut self,
        start: usize,
        count: usize,
    ) -> Result<(), FreeError> {
        let end = start.checked_add(count).ok_or(FreeError::InvalidRange)?;
        if count == 0 || end > self.page_count {
            return Err(FreeError::InvalidRange);
        }

        // 先完整检查，再修改，避免只释放一半后才发现 double free。
        if (start..end).any(|page| !self.is_used(page)) {
            return Err(FreeError::DoubleFree);
        }

        for page in start..end {
            self.set_used(page, false);
        }
        Ok(())
    }
}
```

### 逐段讲解

- 一个 bit 代表一个虚拟页是否已被占用；bit 为 1 表示占用，0 表示空闲。
- `page / 64` 找到 bit 所在的 u64；`page % 64` 找到这个 u64 中的 bit 位。
- First Fit 从低页号开始扫描，找到第一段长度满足 `count` 且起点满足 `align_pages` 的连续空闲区间。
- `reserve` 用于把内核映像、保护页或其他已占用虚拟区间标记为不可分配。
- `deallocate` 先检查整段区间确实全部已占用，再一次性清零，避免发生部分释放。

### 课堂中的空间计算

管理 4096 个页面时：

- `[bool; 4096]` 通常占 4096 bytes。
- bitset 只需 4096 bits，也就是 512 bytes。

所以 bitset 可以把这部分状态元数据减少到原来的八分之一。若 Bitmap 是局部变量，它通常进入当前函数栈帧；若作为分配器字段，则存放位置由分配器实例的存放位置决定。

### 如何运行真实测试

可在仓库中执行：

```bash
cargo test --manifest-path labs/11-bitmap-allocator/Cargo.toml
```

该实验只验证虚拟页区间分配算法，不会修改真实 CPU 页表，也不会分配物理页帧。它与物理页帧分配器、Mapper 是三个不同层次。
