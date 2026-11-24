# 附录：macOS 开发环境与 QEMU

本课程使用 macOS 作为开发环境，以 Rust 编写 `no_std` 内核，并通过 QEMU 运行 x86_64 实验。

## 基础工具

建议准备 Rust 工具链、QEMU、`greadelf` / `readelf`、`objdump` 和 mdBook。

通过 Homebrew 安装常见工具：

```bash
brew install qemu binutils
```

安装 mdBook：

```bash
cargo install mdbook
```

检查：

```bash
rustc --version
cargo --version
qemu-system-x86_64 --version
greadelf --version
mdbook --version
```

不同 macOS 和 Homebrew 版本中，GNU binutils 工具的命令名可能带有 `g` 前缀，例如 `greadelf`、`gobjdump`。

## Rust target 与 Bootloader

RustOS 使用的 target、Bootloader crate 和构建命令取决于项目当前采用的启动方案。不要直接照搬其他教程的 target JSON 或 `bootimage` 命令。

先检查项目配置：

```bash
rustup show
rustc --print target-list | grep x86_64
cat Cargo.toml
```

再根据当前 Bootloader 版本的文档配置 target、链接参数和构建流程。

## QEMU

QEMU 可以模拟 CPU、内存和设备，适合验证内核入口、串口日志、页表和异常处理。QEMU 参数要与当前镜像格式匹配。ISO、磁盘镜像、裸二进制和 Bootloader 生成的镜像，其启动参数可能不同。

## 常见调试方式

- 串口输出：把内核日志输出到 QEMU 串口。
- ELF 检查：`greadelf -h -l kernel.elf` 查看 ELF Header 和 Program Headers。
- 反汇编：`gobjdump -d kernel.elf` 查看机器指令。
- QEMU GDB stub：需要时使用 `-s -S` 暂停 CPU 并等待 GDB 连接；具体命令需结合启动脚本。

## mdBook 本地预览

在仓库根目录：

```bash
mdbook serve book --open
mdbook build book
```

输出目录为 `book/book/`，该目录是构建产物，默认由 `.gitignore` 排除。
