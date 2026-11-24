# RustOS Labs

本目录保存与课程章节对应的代码。每个实验都标明它是独立可测试的算法、可编译的内核模块，还是需要完整内核上下文才能运行的实验。

## 实验目录

| 目录 | 内容 | 验证方式 |
|---|---|---|
| 03-kernel-entry/ | no_std 内核入口、BootInfo、panic handler | cargo check --target x86_64-unknown-none |
| kernel-memory/ | Free List 物理页帧分配器、OffsetPageTable 初始化、map/unmap | target check；需要集成进真实内核才能运行 |
| 11-bitmap-allocator/ | Bitmap + First Fit 虚拟页区间分配 | cargo test |

## 环境

需要 Rust stable 和 x86_64 bare-metal target：

~~~bash
rustup toolchain install stable --profile minimal
rustup target add x86_64-unknown-none
~~~

## 运行实验

最小内核入口：

~~~bash
cargo check --manifest-path labs/03-kernel-entry/Cargo.toml \
  --target x86_64-unknown-none
~~~

内存管理模块：

~~~bash
cargo check --manifest-path labs/kernel-memory/Cargo.toml \
  --target x86_64-unknown-none
~~~

Bitmap + First Fit：

~~~bash
cargo test --manifest-path labs/11-bitmap-allocator/Cargo.toml
~~~

## 验证声明

代码能够通过 cargo check 只说明语法、类型和目标依赖可以编译，不代表它已经在 QEMU 中安全运行。Free List 和 Mapper 依赖 Bootloader 提供的物理内存直接映射；AddressSpace / CR3 切换还依赖正确的内核映射和栈布局。

第 9 课的映射/回收整合实验曾明确暂缓。本目录不会把它标记为已完成。
