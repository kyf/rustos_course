# 附录：代码来源、验证状态与阅读约定

本书是从此前的 RustOS 课程整理而来。为了避免把“曾经讲过”“为了教材重新整理”和“已经运行验证”混为一谈，所有代码按以下来源标记。

## 来源标记

| 标记 | 含义 | 能否称为历史原代码 |
|---|---|---|
| **历史授课片段** | 历史对话中确实出现过的代码骨架、依赖版本或命令；可能做了排版和命名整理 | 可以，但只限明确标出的片段 |
| **按历史设计重构** | 历史课程中明确讲过实现方案，但无法从历史记录逐字恢复完整代码；现在按相同设计补成代码 | 不可以，必须说明是重构 |
| **新增补充示例** | 为了把原理讲完整而新增的演示代码 | 不可以 |
| **未验证** | 代码尚未在本仓库对应内核工程中编译或运行 | 不可以称为已完成实验 |
| **CI 验证** | 当前仓库的 GitHub Actions 已经执行相应构建或测试并成功 | 仅对该 CI 实际执行的目标有效 |

## 课程环境与版本

历史课程中曾使用：

- **bootloader_api = 0.11.17**
- **x86_64 = 0.15.2**，后因当前工具链下 trait 方法不匹配问题升级到 **0.15.5**
- **uart_16550 = 0.4.0**
- **x86_64-unknown-none** 作为内核编译目标
- QEMU 作为 x86_64 虚拟机

课程代码示例默认以 bootloader_api 0.11.17 和 x86_64 0.15.5 为主要 API 参照。依赖版本变更必须经过编译验证，不能只根据其他教程的代码推断兼容性。

## 原课程课次与本书章节

本书为了让知识依赖更清晰，把内容重新编排为 00–16 章。它们不等于历史课程的原始课次。

| 历史学习课次 | 本书对应主题 | 记录 |
|---|---|---|
| 第 1–3 课 | OS 基础、CPU/栈、启动入口 | 早期概念与最小内核入口代码 |
| 第 4–6 课 | 虚拟内存、页表、对齐与页帧 | 讨论了 RIP/RSP/CR3、BootInfo、物理内存偏移、页表索引和对齐计算 |
| 第 7–8 课 | 页表映射、解除映射与内存管理接口 | 以 Mapper / map_to / unmap / TLB 为主线 |
| 第 9 课 | 映射与回收整合实验 | **实验曾明确暂缓**，待与物理页帧回收、虚拟地址分配统一整合 |
| 第 10 课 | PhysicalFrameAllocator | 历史方案为 Free List / 侵入式链表，空闲页帧前 8 字节存 next 物理地址 |
| 第 11 课 | VirtualAddressAllocator | Bitmap、First Fit、连续虚拟页分配与释放 |
| 第 12 课 | Process / Thread | 用户栈、内核栈、线程上下文和 CR3 的归属 |
| 第 13 课 | AddressSpace | 新 PML4、共享内核映射、CR3 切换与相同 VA 不同 PA 的隔离验证目标 |

该表是对已找回课程记录的归纳，不代表每个课次的全部源码都已逐字恢复。

## 当前代码目录

- **labs/03-kernel-entry/**：最小 no_std 内核入口，可单独进行 target check。
- **labs/kernel-memory/**：Free List 页帧分配器和 Mapper 代码模块，供内核工程集成；它不是完整可启动内核。
- **labs/11-bitmap-allocator/**：纯 Rust Bitmap + First Fit 算法，带宿主机单元测试。

## 当前验证边界

CI 会执行：
1. mdBook 构建；
2. Bitmap 分配器单元测试；
3. 最小内核入口在 x86_64-unknown-none target 上的检查；
4. kernel-memory 模块在 x86_64-unknown-none target 上的检查。

这些检查不能证明真实 QEMU 启动、物理页帧映射、CR3 切换或多核同步已经完成。那些实验必须在完整内核工程中运行并记录真实输出后，才能标记为“QEMU 已验证”。

## 外部 API 参考

- [bootloader_api 0.11.17：entry_point!](https://docs.rs/bootloader_api/0.11.17/bootloader_api/macro.entry_point.html)
- [bootloader_api 0.11.17：BootInfo 与 MemoryRegion](https://docs.rs/bootloader_api/0.11.17/bootloader_api/info/struct.BootInfo.html)
- [x86_64 0.15.5：OffsetPageTable](https://docs.rs/x86_64/0.15.5/x86_64/structures/paging/mapper/struct.OffsetPageTable.html)
- [x86_64 0.15.5：FrameAllocator / FrameDeallocator](https://docs.rs/x86_64/0.15.5/x86_64/structures/paging/index.html)
- [uart_16550 0.4.0](https://docs.rs/uart_16550/0.4.0/uart_16550/)
