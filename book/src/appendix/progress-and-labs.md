# 附录：学习进度与实验状态

本页用于区分已经学习的原理、已经完成的实验和仍待实现的内核功能。状态以学习记录为准，后续应随代码仓库进展更新。

## 已经学习并讨论的主题

- 操作系统的资源管理、隔离、调度和抽象职责。
- Ring 0 / Ring 3、系统调用与普通 OS API 的区别。
- RIP、RSP、寄存器、函数调用、栈和线程上下文。
- BIOS/UEFI、Bootloader、ELF、BIN、ROM 与 BootInfo。
- QEMU 基础使用、内核构建脚本、Cargo 环境变量。
- 虚拟地址、物理地址、MMU、TLB、PML4/PDPT/PD/PT 和 CR3。
- 页与页帧、4 KiB 对齐、`align_up` / `align_down`。
- BootInfo MemoryRegion、Usable、Bootloader、UnknownBios。
- 物理页帧分配、直接映射、`Page` / `PhysFrame` 和 PageTableFlags。
- `map_to`、`unmap`、TLB flush、页帧回收和资源所有权。
- Bitmap、First Fit、规范虚拟地址、连续虚拟区间与不连续物理页帧。
- 进程/线程、用户栈/内核栈、CR3 与独立地址空间。

## 已经进行过的实验（根据学习记录）

- 使用 QEMU 启动 Rust 内核并查看 ELF / Program Headers。
- 输出并分析 BootInfo 中的 MemoryRegion。
- 根据页对齐规则计算可分配完整页帧数量。
- 实现或练习物理页帧申请、虚拟地址分配、映射/解除映射和地址读写。
- 讨论并准备独立页表根、相同虚拟地址映射不同物理页帧的实验。

注意：本教材没有附上所有历史项目源码和原始实验日志。除非对应代码已经放进 `labs/` 并在当前环境验证，否则不要把章节中的伪代码当作可直接运行的完整实现。

## 尚未完成的核心实现

- 完整 Ring 3 用户程序加载与运行。
- 稳定的系统调用入口、参数校验与返回路径。
- IDT 中断/异常处理、定时器中断。
- 可抢占调度器和线程上下文切换。
- 进程创建/销毁、地址空间资源回收和多核 TLB shootdown。
- 设备驱动、块设备、文件系统和 Shell。

## 实验代码整理规范

每个 `labs/<topic>/` 目录应包含：

- `README.md`：目标、依赖版本、运行方式、预期结果。
- 源码：可以独立构建的实验代码，或明确说明如何应用到内核主项目。
- `expected-output.txt`：关键日志或验证结果。
- `notes.md`：实现中遇到的问题、原因与修复。

课程正文可以保留原理解释；只有经过构建或 QEMU 验证的代码才标记为“已验证实验”。
