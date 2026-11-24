# 附录：术语表

| 术语 | 含义 |
|---|---|
| ABI | Application Binary Interface，二进制层面的调用约定与接口规则 |
| Address Space | 地址空间；某个执行上下文可使用的虚拟地址集合及其映射 |
| Bootloader | 引导加载程序，负责加载内核并准备启动环境 |
| BootInfo | Bootloader 传递给内核的启动信息 |
| CR3 | x86_64 控制寄存器，包含当前页表根的物理地址及相关控制字段 |
| ELF | Executable and Linkable Format，可执行与可链接文件格式 |
| Frame | 物理页帧 |
| GDT | Global Descriptor Table，全局描述符表 |
| IDT | Interrupt Descriptor Table，中断描述符表 |
| MMU | Memory Management Unit，内存管理单元 |
| PML4 | 四级分页模式下的最高级页表 |
| Page | 虚拟页 |
| Page Fault | 页故障异常，访问地址或权限不符合页表规则时产生 |
| PA | Physical Address，物理地址 |
| PCID | Process-Context Identifier，用于区分 TLB 中不同地址空间上下文 |
| QEMU | 机器模拟器和虚拟化工具 |
| Ring 0 / Ring 3 | x86 特权级概念，常用于表示内核态与用户态 |
| RIP | 指令指针寄存器 |
| RSP | 栈指针寄存器 |
| TLB | Translation Lookaside Buffer，地址转换缓存 |
| VA | Virtual Address，虚拟地址 |
| VMM | Virtual Memory Manager，虚拟内存管理器 |
| no_std | Rust 不链接标准库的运行模式，常用于内核和裸机程序 |
