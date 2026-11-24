# RustOS Course

一套使用 Rust 从零理解并逐步实现操作系统的中文教程。

本教程以 **x86_64、Rust、`no_std` 与 QEMU** 为主线，强调底层原理，并通过逐步增加的实验实现一个具备基础操作系统特征的教学内核。

## 本地阅读

```bash
cargo install mdbook
git clone https://github.com/kyf/rustos_course.git
cd rustos_course
mdbook serve book --open
```

构建静态网页：`mdbook build book`，输出位于 `book/book/`（由 `book/book.toml` 配置）。

## 教程范围

本阶段整理了此前已经学习的内容：操作系统职责、CPU 执行与栈、启动链路、ELF 与 Bootloader、QEMU、BootInfo 与物理内存地图、虚拟内存、x86_64 多级页表、CR3、物理页帧分配、直接映射、页表映射与撤销、TLB、虚拟地址分配、进程/线程、独立地址空间，以及 Ring 3 和系统调用入口的原理预备。

课程按知识依赖重新组织，并非聊天记录逐字转录。凡是原始代码或实验日志没有在当前教材中完整保存的地方，都会明确标记为原理说明或待补实验，不会将伪代码说成已经验证的实现。

## 当前进度

- [x] 操作系统职责、用户态/内核态与系统调用概念
- [x] CPU 寄存器、指令执行、函数调用与栈
- [x] Bootloader、ELF、BootInfo 与启动交接
- [x] QEMU、构建脚本与 Cargo 构建环境
- [x] 虚拟内存、分页、PML4/PDPT/PD/PT 与 CR3
- [x] 物理页帧分配、物理内存直接映射
- [x] 页表映射/撤销、TLB 与资源回收
- [x] 虚拟地址分配、Bitmap 与 First Fit
- [x] 进程/线程、栈与独立地址空间的原理和初步设计
- [ ] Ring 3 实际切换、系统调用实现与用户程序运行
- [ ] 中断控制、抢占式调度与多核同步
- [ ] 驱动、文件系统与 Shell

## 课程约定

- 先解释机制，再讨论策略，最后实现抽象。
- 区分硬件机制、内核策略和 Rust API。
- 每章标注“已讨论”“已实验”或“待实现”，避免混淆学习进度。
- 代码示意不等于可直接运行的完整内核代码；具体 API 以仓库锁定的 crate 版本为准。
