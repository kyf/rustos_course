# 14. Rust 内核构建流程与 QEMU 调试

## 本课定位

这一章整理此前学习中遇到的 `build.rs`、Cargo 环境变量、ELF 产物和 QEMU 使用问题。它解释构建链路，不绑定某一个特定版本的 Bootloader crate。

## 从 Cargo 到 QEMU 的构建链路

典型流程可以概括为：

```text
Cargo 解析 workspace 与依赖
        |
运行 build.rs（生成配置/链接参数/资源文件）
        |
编译 Rust crate 与内核目标
        |
链接生成 ELF
        |
Bootloader 或镜像工具包装/加载内核
        |
QEMU 启动虚拟机器并执行启动链路
```

ELF 适合调试和符号分析；某些启动流程会进一步把内核打包进磁盘镜像、启动镜像或原始二进制。具体产物由项目的 Bootloader 和构建方案决定。

## `build.rs` 是什么时候运行的

`build.rs` 是 Cargo 的构建脚本，在 Cargo 编译当前 package 的目标之前运行。它常用于：

- 检查环境变量或目标平台。
- 生成 Rust 源码、配置文件或链接器脚本。
- 编译 C/汇编辅助代码。
- 向 Cargo 输出链接参数和重跑条件。

常见输出指令包括：

```rust
println!("cargo:rerun-if-changed=build.rs");
println!("cargo:rerun-if-env-changed=MY_KERNEL_CONFIG");
println!("cargo:rustc-link-search=native=/some/path");
println!("cargo:rustc-link-lib=static=my_support");
```

这些 `cargo:` 行是写给 Cargo 的构建指令，不是普通程序运行时日志。`build.rs` 的 `println!` 通常出现在 Cargo 构建输出中，而不是内核启动后的 QEMU 串口。

## 常见 Cargo 环境变量

| 环境变量 | 含义 |
|---|---|
| `CARGO_MANIFEST_DIR` | 当前 package 的 `Cargo.toml` 所在目录 |
| `OUT_DIR` | 当前 build script 可写生成文件的目录 |
| `TARGET` | 正在编译的目标 triple |
| `HOST` | 运行构建脚本的主机 triple |
| `PROFILE` | 当前构建 profile，例如 `debug` 或 `release` |
| `OPT_LEVEL` | 优化级别 |
| `DEBUG` | 当前 profile 是否启用调试信息的相关配置值 |
| `CARGO_PKG_NAME` / `CARGO_PKG_VERSION` | 当前 package 名称/版本 |
| `CARGO_CFG_TARGET_*` | 目标平台配置，例如架构、操作系统、指针宽度等 |

并不是所有变量在所有 Cargo 阶段都可用；应以 Cargo Reference 对应版本的说明为准。

### `CARGO_BIN_FILE_*` 是什么

`CARGO_BIN_FILE_<name>` 这一类变量与 Cargo 对依赖项声明的二进制 artifact 有关。它可以让使用方取得某个依赖二进制产物的路径，具体变量名由依赖目标名称等规则决定。它不是“当前 package 任意编译产物”的通用变量。

如果 `build.rs` 需要当前 package 自己最终生成的内核 ELF，要注意构建顺序：build script 在当前 package 的二进制目标完成之前运行，因此不能把它当成可靠的“当前目标最终 ELF 路径”接口。常见解决方式是把内核作为独立 package，再由包装 package 通过 Cargo artifact 依赖它；或者在 Cargo 构建完成后由外层脚本执行镜像打包步骤。

## QEMU 在这里做什么

QEMU 模拟一台机器，包括虚拟 CPU、RAM 和设备。我们可以在不烧录真实硬件的情况下验证启动流程、串口、异常和内存管理机制。它不是单纯的“运行 ELF 的程序”：镜像格式、固件、Bootloader 和机器类型必须匹配。

常见调试参数（是否适用取决于启动脚本）：

```bash
# 将串口重定向到当前终端
-serial stdio

# 发生关机/重启时不要自动循环重启，便于观察故障
-no-reboot -no-shutdown

# 开启 GDB stub 并在 CPU 启动前暂停
-s -S
```

QEMU 的 `-m` 用来设置虚拟机 RAM 大小。改变 `-m` 后，BootInfo 中的内存区域也可能变化；但不能把所有 MemoryRegion 的 size 简单相加当作可用 RAM，必须看区域类型。

## 查看 ELF

```bash
greadelf -h kernel.elf
greadelf -lW kernel.elf
gobjdump -d kernel.elf
```

- `-h` 查看 ELF Header，例如文件类型、架构和入口地址。
- `-lW` 查看 Program Headers，重点关注 `LOAD` 段的文件偏移、虚拟地址、文件大小、内存大小和权限。
- `objdump -d` 查看反汇编。

`Entry point` 是 ELF 声明的入口值；它不自动保证 Bootloader 已经建立了入口代码所需的栈、页表映射和 CPU 状态。

## 实验建议

1. 在项目 `build.rs` 中打印 `CARGO_MANIFEST_DIR`、`OUT_DIR`、`TARGET` 和 `PROFILE`，观察构建阶段变量。
2. 使用 `greadelf -h -lW` 检查内核 ELF，记录入口地址和 LOAD 段。
3. 使用项目原有启动脚本运行 QEMU，区分 Cargo 构建输出与内核串口输出。
4. 改变 QEMU RAM 大小，比较 BootInfo 内存区域变化。

## 本课结论

Cargo 构建脚本、ELF 链接产物、Bootloader 镜像和 QEMU 启动是不同阶段。定位问题时，先确认错误发生在哪个阶段，再检查该阶段的输入与输出。


## 代码演示：build.rs 的职责

\`build.rs\` 在 Cargo 编译当前 package 的目标之前运行。它可以生成配置文件、检查环境变量或向 Cargo 提供链接参数；它不能可靠地读取当前 package 尚未生成的最终 ELF。

\`\`\`rust
// build.rs
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-env-changed=RUSTOS_KERNEL_FEATURE");

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir")
    );
    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR").expect("out dir")
    );

    let linker_script = manifest_dir.join("linker.ld");
    if linker_script.exists() {
        let copied = out_dir.join("linker.ld");
        fs::copy(&linker_script, &copied).expect("copy linker script");
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rustc-link-arg=-T{}", copied.display());
    }
}
\`\`\`

### 为什么不能在 build.rs 里直接拿当前 ELF

Cargo 先运行当前 package 的 build script，再编译该 package 的 library/binary targets。因此当前 package 的最终 ELF 此时还没有生成。若某个外部工具需要最终内核 ELF，通常应放在构建完成后的外层脚本，或者把内核拆成一个独立 package，再通过 Cargo artifact dependency 传递产物路径。

\`CARGO_BIN_FILE_*\` 不是“当前 package 任意最终产物”的通用变量。它用于 Cargo 为依赖的二进制 artifact 提供文件路径；需要根据具体依赖声明和 Cargo 版本确认变量名。

## 代码演示：QEMU 串口与 GDB 调试

只有在 Bootloader 已经把内核打包成可启动镜像后，才把镜像路径传给 QEMU。下面的 \`$IMAGE\` 是占位变量，必须替换成项目实际生成的 BIOS/UEFI 镜像；它不是内核 ELF 路径。

\`\`\`bash
qemu-system-x86_64 \\
  -machine q35 \\
  -m 512M \\
  -serial stdio \\
  -no-reboot \\
  -no-shutdown \\
  -drive format=raw,file="$IMAGE" \\
  -s -S
\`\`\`

- \`-m 512M\` 设置虚拟机 RAM 容量。
- \`-serial stdio\` 把串口输出接到当前终端。
- \`-no-reboot -no-shutdown\` 让故障发生后保留现场。
- \`-s\` 开启 GDB stub（默认 TCP 1234）。
- \`-S\` 在 CPU 执行第一条指令前暂停，等待 GDB 连接。

另开终端：

\`\`\`bash
gdb target/x86_64-unknown-none/release/rustos-kernel
\`\`\`

在 GDB 中：

\`\`\`gdb
target remote :1234
break kernel_main
continue
info registers
x/12i $rip
x/16gx $rsp
\`\`\`

ELF 路径要与实际产物一致。若调试信息和符号存在，GDB 才能把地址解析成函数和源代码行。

### 本课程里如何区分输出

- Cargo / build.rs 输出：构建阶段日志。
- QEMU 终端上的串口输出：内核启动后由 UART 驱动发送的数据。
- GDB 输出：调试器读取的 CPU 寄存器、内存和符号信息。

把 \`build.rs\` 的 \`println!\` 当成内核串口输出，是定位日志问题时常见的混淆。
