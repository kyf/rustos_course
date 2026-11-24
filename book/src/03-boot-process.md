# 03. 从上电到内核入口：启动链路

## 启动链路

典型 PC 启动过程可以概括为：

```text
CPU 复位
  |
固件（BIOS / UEFI）
  |
启动设备与 Bootloader
  |
加载内核镜像并准备运行环境
  |
跳转到内核入口
  |
内核初始化
```

BIOS/UEFI 是平台固件。Bootloader 通常是单独的软件组件，负责选择并加载操作系统内核。它不是 BIOS/UEFI 的同义词，也不一定是固件本身的一部分。

## 为什么“加载了 ELF”还不代表内核能运行

ELF 文件描述程序入口、程序段和相关属性。但 CPU 要执行内核，还需要合适的运行环境：

- CPU 处于内核预期的执行模式。
- 栈已经建立并可访问。
- 内核入口地址有效。
- 入口附近的代码和数据已映射。
- 页表和控制寄存器处于预期状态。
- 中断、描述符表等架构状态符合内核约定。
- Bootloader 按约定传递启动信息。

Bootloader 不只是把 ELF 字节复制到 RAM，还要准备内核能继续执行的环境。

## ELF、BIN 与 ROM

- **ELF**：带结构化元数据的可执行文件格式，包含程序头、段、入口地址等信息。
- **BIN**：通常是原始二进制镜像，不携带完整 ELF 元数据；加载器必须知道它应该放到哪里。
- **ROM**：通常指固件或芯片内置的只读存储代码；模拟器也可能以 ROM 镜像文件形式提供。它不是应用 ELF 的另一种后缀。

实际烧录时，很多流程最终写入 BIN 或带分区/镜像格式的固件，而不是要求设备直接解析开发时的 ELF。ELF 常用于调试、符号解析和加载阶段。

## BootInfo 是什么

BootInfo 是 Bootloader 传递给内核的启动信息结构，可能包含内存区域、帧缓冲区、物理内存偏移等信息。

**BootInfo 本身不是页表。** 它描述启动环境；页表是 CPU 用于地址转换的数据结构。Bootloader 可能已经建立页表，但这与 BootInfo 是两件事。

## QEMU 的作用

QEMU 模拟 x86_64 机器，让我们在不依赖真实硬件的情况下测试内核。它可以提供虚拟 CPU、内存、设备和启动环境，并支持串口输出、调试和异常观察。但 QEMU 能启动不代表所有硬件行为与真实机器完全一致。

## 思考题

1. Bootloader 是否就是 BIOS/UEFI？
2. 为什么 ELF 已经加载到内存，仍然可能无法执行？
3. BootInfo 与页表有什么区别？
4. 如果烧录的是 BIN，设备如何知道镜像应放置的位置？

## 本课结论

启动过程的关键是建立明确的交接协议：Bootloader 准备 CPU 和内存环境，将控制权交给内核，并把启动信息传递给内核。


## CPU 上电时，固件代码在哪里

在常见 x86 PC 上，CPU 复位后会从架构定义的 reset vector 开始取指。CPU 并不是在出厂时就内置了完整的主板 BIOS/UEFI 程序；平台固件通常存放在主板上的 SPI Flash 等非易失存储器中，并由芯片组/平台逻辑映射到 CPU 复位时可访问的地址区域。

所以更准确的描述是：CPU 复位后从固定的架构入口取指，而平台已经把固件映射到该入口可访问的位置。随后固件初始化平台并启动 Bootloader 或其他启动程序。


## 代码演示：最小内核入口

**来源标记：历史授课片段（按已找回的原始代码骨架整理）。** 第 3 课实际展示过 no_std、no_main、entry_point!、BootInfo、kernel_main 和 panic_handler。

~~~rust
#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
~~~

### 逐行解释

1. no_std：不链接 Rust 标准库。内核没有宿主操作系统提供的文件、线程和堆等标准库运行环境。
2. no_main：不使用普通应用程序的 main 启动约定。内核入口由 Bootloader 和链接器约定决定。
3. entry_point!(kernel_main)：宏生成链接器需要的 _start 入口包装函数，并检查入口函数的参数与返回类型。入口函数签名必须是 fn(&'static mut BootInfo) -> !。
4. BootInfo：Bootloader 交给内核的启动信息，不是页表本身。
5. -> !：发散类型，表示函数不会正常返回。内核入口不能返回到一个不存在的调用者。
6. panic_handler：no_std 程序必须提供 panic 处理函数。这里先停在死循环中，后续可以替换成串口打印、记录异常原因或触发调试断点。

### 依赖版本

历史课程曾使用以下依赖版本：

~~~toml
[dependencies]
bootloader_api = "=0.11.17"
x86_64 = "=0.15.5"
uart_16550 = "=0.4.0"
~~~

曾经遇到过 x86_64 = 0.15.2 与当前 Rust 工具链的 trait 方法不匹配（缺少 forward_overflowing / backward_overflowing）的问题；后续通过升级到 0.15.5 解决。不要在没有验证的情况下把依赖版本降回去。

### 把串口作为最早期的调试出口

**来源标记：新增补充示例，API 对应课程记录中的 uart_16550 = 0.4.0。**

~~~rust
use uart_16550::SerialPort;

fn serial_demo() {
    const COM1: u16 = 0x3F8;

    // SAFETY: 假设 QEMU 提供兼容的 COM1，并且当前代码独占访问该端口。
    let mut serial = unsafe { SerialPort::new(COM1) };
    serial.init();

    serial.send(b'R');
    serial.send(b'u');
    serial.send(b's');
    serial.send(b't');
    serial.send(10);
}
~~~

这里的 unsafe 是因为创建 UART 对象意味着承诺 I/O 端口地址有效且没有另一个驱动同时控制同一设备。串口输出只有在 UART 初始化完成、QEMU 把串口重定向到终端时才看得见。

### 让 Bootloader 提供物理内存直接映射

后续页表实验需要通过物理内存偏移访问页表页帧。Bootloader 0.11 的配置可以通过入口宏传入：

~~~rust
use bootloader_api::{
    config::{BootloaderConfig, Mapping},
    entry_point,
    BootInfo,
};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
~~~

Mapping::Dynamic 的含义是 Bootloader 在运行时寻找一个可用虚拟地址范围来映射物理内存；它不是“每分配一个物理页帧就自动新建一个映射”。启用后，BootInfo 的 physical_memory_offset 才可能是 Some(offset)。如果该字段是 None，内核不能假设直接映射存在。

### 构建入口 ELF

~~~bash
rustup target add x86_64-unknown-none
cargo build --release --target x86_64-unknown-none
~~~

这一步构建的是内核 ELF，不等于已经生成可启动磁盘镜像。Bootloader 的 BIOS/UEFI 镜像打包步骤由外层 bootloader 工程或项目脚本完成；不能把内核 ELF 直接当成 QEMU 磁盘镜像传入。

> 完整、可独立检查的最小入口工程见 [labs/03-kernel-entry](https://github.com/kyf/rustos_course/tree/docs/course-code-examples/labs/03-kernel-entry)。
