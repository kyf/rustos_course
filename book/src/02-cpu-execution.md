# 02. CPU、寄存器、栈与指令执行

## 为什么需要寄存器

CPU 执行指令时需要频繁读取和写入操作数。如果每一步都访问 RAM，通常会带来较高延迟。寄存器是 CPU 内部数量有限、访问很快的存储位置，用来保存当前计算所需的数据和控制状态。

寄存器可粗略分为通用寄存器、指令指针、栈指针、标志寄存器和控制寄存器。

## RIP 与 RSP

在 x86_64 中：

- `RIP`：Instruction Pointer，指令指针。
- `RSP`：Stack Pointer，栈指针。

CPU 根据 RIP 找到下一条指令并执行。指令执行后，RIP 通常前进；遇到跳转、调用或返回时，RIP 会被控制流指令改变。

RSP 指向当前栈顶附近的位置。栈通常位于内存中，常用于保存返回地址、局部数据和调用约定要求保存的寄存器。

## call 与 ret

概念上，近调用 `call target` 会将返回地址压入当前栈，并把 RIP 改为目标地址。`ret` 从栈顶取出返回地址并放回 RIP。

重要：`call` 保存的是返回地址，不是自动保存全部寄存器。哪些寄存器由调用者保存、哪些由被调用者保存，取决于 ABI（调用约定）。

## 栈为什么不能随意共享

每个线程通常需要自己的执行栈。假设两个线程共用一个栈，它们的函数调用会交错修改栈顶、返回地址和局部变量，导致数据互相覆盖，最终可能跳转到错误地址。

同一进程的线程通常共享地址空间，但每个线程有独立栈和独立执行上下文。

## 一条 add 指令发生什么

以 `add rax, rbx` 为例：

1. CPU 读取 RAX 和 RBX。
2. 执行整数加法。
3. 将结果写回 RAX。
4. 根据结果更新相关标志位，例如零标志、进位标志和溢出标志。

## 思考题

1. 为什么寄存器数量有限，但仍然比直接使用 RAM 更适合保存当前计算状态？
2. `call` 会自动保存所有通用寄存器吗？为什么？
3. 如果两个线程共用同一个栈，可能发生什么？
4. `RIP` 和 `RSP` 分别解决什么问题？

## 本课结论

程序执行本质上是 CPU 不断读取指令、更新寄存器和访问内存的过程。函数调用、返回和线程切换，都离不开对控制流状态与栈状态的管理。


## 代码演示：从 Rust 里读取 RIP、RSP 和 CR3

**来源标记：新增补充示例。** 这段代码把我们课堂上讨论的寄存器变成可观察的数据。它必须在 x86_64 内核态执行；尤其读取 CR3 是特权操作，不能在普通 macOS 用户态程序中运行。

\`\`\`rust
use core::arch::asm;

/// 读取当前函数执行时的栈指针。
#[inline]
pub fn read_rsp() -> u64 {
    let value: u64;
    unsafe {
        asm!(
            "mov {}, rsp",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// 取得执行这条 LEA 指令时的 RIP 相对位置。
#[inline]
pub fn read_rip() -> u64 {
    let value: u64;
    unsafe {
        asm!(
            "lea {}, [rip + 0]",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// 读取当前页表根寄存器 CR3。
/// 只能在有权读取控制寄存器的内核态执行。
#[inline]
pub unsafe fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        asm!(
            "mov {}, cr3",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}
\`\`\`

### 逐段解释

- \`asm!\` 是 Rust 内联汇编宏。汇编本身不由 Rust 类型系统验证，所以必须由调用者保证指令适用于当前架构和特权级。
- \`out(reg) value\` 让编译器选择一个通用寄存器，并把汇编输出写入 Rust 变量。
- \`mov {}, rsp\` 读取当前 RSP；它不会创建新栈，也不会分配内存。
- \`lea {}, [rip + 0]\` 使用 RIP-relative addressing 计算当前指令附近的地址。它不是读取“程序入口地址”，而是读取当前执行位置附近的指令地址。
- \`mov {}, cr3\` 读取当前逻辑 CPU 正在使用的页表根信息。CR3 的低位还可能包含缓存控制或 PCID 相关信息，不能把整个数值都直接当作 PML4 物理基址。

### 怎么验证

把这些函数放进内核代码，在串口日志中输出十六进制值。RSP 应当落在当前内核栈映射范围内；CR3 中对齐后的页表根地址应当满足 4 KiB 对齐。不要只根据“数值看起来很大”判断它是虚拟地址还是物理地址。

### 函数调用与栈帧

\`\`\`rust
#[inline(never)]
fn add_and_scale(a: u64, b: u64) -> u64 {
    let sum = a + b;
    sum * 2
}

#[inline(never)]
fn caller() -> u64 {
    let left = 10;
    let right = 20;
    add_and_scale(left, right)
}
\`\`\`

编译器可能把参数放在 ABI 规定的寄存器中，也可能根据优化等级改变栈帧布局。要观察真实指令，应查看目标 ELF 的反汇编，而不是假定每个局部变量都一定在栈上：

\`\`\`bash
gobjdump -d -Mintel target/x86_64-unknown-none/release/rustos_kernel
\`\`\`

**本例要建立的直觉：** RSP 是执行流当前使用的栈位置；RIP 是当前指令流的位置；函数调用至少要保存返回地址，但并不自动保存所有通用寄存器。
