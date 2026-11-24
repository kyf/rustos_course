# 附录：学习进度与实验状态

本页区分四件事：已经讲授的概念、历史课程中曾经出现的代码、当前仓库新增的补充代码，以及真正经过构建或 QEMU 验证的实验。它们不能互相替代。

## 已经学习并讨论的主题

- 操作系统的资源管理、隔离、调度和抽象职责。
- Ring 0 / Ring 3、系统调用与普通 OS API 的区别。
- RIP、RSP、寄存器、函数调用、栈和线程上下文。
- BIOS/UEFI、Bootloader、ELF、BIN、ROM 与 BootInfo。
- QEMU 基础使用、内核构建脚本、Cargo 环境变量。
- 虚拟地址、物理地址、MMU、TLB、PML4/PDPT/PD/PT 和 CR3。
- 页与页帧、4 KiB 对齐、align_up / align_down。
- BootInfo MemoryRegion、Usable、Bootloader、UnknownBios。
- 物理页帧分配、直接映射、Page / PhysFrame 和 PageTableFlags。
- map_to、unmap、TLB flush、页帧回收和资源所有权。
- Bitmap、First Fit、规范虚拟地址、连续虚拟区间与不连续物理页帧。
- 进程/线程、用户栈/内核栈、CR3 与独立地址空间。

## 已找回的历史代码线索

- 第 3 课曾展示最小内核入口：no_std、no_main、bootloader_api 的 entry_point!、BootInfo、kernel_main 和 panic_handler。
- 第 4 课使用过 bootloader_api 0.11.17、x86_64 0.15.2 和 uart_16550 0.4.0；后续因 x86_64 0.15.2 与工具链 API 不匹配，升级到 0.15.5。
- 第 10 课曾讲解 PhysicalFrameAllocator 的 Free List 设计：空闲页帧前 8 字节存放 next 物理地址；分配器对象自身保存 free_head、physical_memory_offset 和 free_frames；实现 FrameAllocator / FrameDeallocator。
- 第 11 课曾讲解 Bitmap + First Fit 虚拟地址分配，包含连续页面分配、对齐、释放、bool 数组与 bitset 的元数据大小比较。
- 第 12 课讨论了 Process / Thread、用户栈与内核栈、线程切换和 CR3 的归属。
- 第 13 课的目标是创建新 PML4、共享必要内核映射、切换 CR3，并用两个地址空间的相同 VA 映射不同 PA 来验证隔离。

无法从历史记录逐字恢复的完整实现，已经标注为“按历史设计重构”，不会冒充历史原代码。详细来源说明见 [代码来源与验证状态](code-status.md)。

## 实验状态

### 已经进行过的验证（根据历史学习记录）

- 使用 QEMU 启动 Rust 内核并查看 ELF / Program Headers。
- 输出并分析 BootInfo 中的 MemoryRegion。
- 根据页对齐规则计算可分配完整页帧数量。
- 观察过内核构建、启动和调试过程中的实际日志。

### 曾讨论或编写，但不能据此宣称 QEMU 已验证

- PhysicalFrameAllocator Free List 的完整生命周期。
- Mapper 的 map_to / unmap 与 TLB flush。
- Bitmap + First Fit 虚拟地址分配器。
- 独立 PML4、内核映射共享和 CR3 切换。
- A/B 地址空间相同 VA、不同 PA 的隔离实验。

### 明确暂缓的实验

**历史第 9 课的实验曾明确暂缓**，原因是希望把页表映射、物理页帧回收和虚拟地址分配放到统一的内存管理流程中验证。当前文档中的 TLB 章节解释了机制，但不能把它等同于第 9 课整合实验已经完成。

## 当前仓库新增的代码

- labs/03-kernel-entry：最小 no_std 内核入口工程。
- labs/kernel-memory：Free List 页帧分配器和 OffsetPageTable / map_to / unmap 模块。
- labs/11-bitmap-allocator：Bitmap + First Fit 算法及宿主机单元测试。

这些代码是为本书补充的工程化材料。GitHub Actions 通过后，只能说明对应 target check 或单元测试通过；不代表这些模块已经集成到一个完整内核，也不代表物理内存、页表切换或 QEMU 实验已完成。

## 尚未完成的核心实现

- 完整 Ring 3 用户程序加载与运行。
- 稳定的系统调用入口、参数校验与返回路径。
- IDT 中断/异常处理、定时器中断。
- 可抢占调度器和线程上下文切换。
- 进程创建/销毁、地址空间资源回收和多核 TLB shootdown。
- 将物理页帧分配、虚拟地址分配、映射/撤销映射和失败回滚整合为统一 MemoryManager。
- 设备驱动、块设备、文件系统和 Shell。

## 代码整理规范

每个 `labs/<topic>/` 目录应包含：

- README.md：目标、依赖版本、运行方式、预期结果。
- 源码：可以独立构建的实验代码，或明确说明如何应用到内核主项目。
- expected-output.txt：真实运行后的关键日志；没有运行就不要伪造。
- notes.md：实现中遇到的问题、原因与修复。

只有经过构建验证的代码才标记为“CI 验证”；只有在 QEMU 实际运行并记录结果后，才标记为“QEMU 已验证”。
