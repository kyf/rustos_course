# 教材文档贡献规范

本仓库使用 mdBook 编写 RustOS 教程。新增或修改章节时，必须遵守以下 Markdown 与代码示例约定。

## 1. Rust 代码块

- 所有多行 Rust 代码必须使用标准 Markdown fenced code block，并标注语言：

  ```markdown
  ```rust
  fn main() {
      println!("hello");
  }
  ```
  ```

- 不要对代码围栏中的反引号添加反斜杠转义。围栏必须由三个或更多连续的反引号（或成对的波浪号围栏）组成，并且必须正确闭合。
- Shell 命令使用 `bash`，纯文本输出使用 `text`，配置示例按实际语法标注语言。
- 不要把代码示例写成普通段落；代码块内部的 Rust 泛型尖括号不需要转义。

## 2. 正文中的类型、泛型和路径

- 正文提到 Rust 类型、函数、变量或表达式时，使用行内代码，例如 `Option<VirtAddr>`、`PhysFrame<Size4KiB>`、`FrameAllocator<Size4KiB>`。
- 路径模板或带尖括号的占位符也使用行内代码，例如 `labs/<topic>/`。
- 不要让普通 Markdown 正文裸露类似 HTML 标签的泛型文本，例如 `<VirtAddr>`。否则 mdBook 可能将其解释为 HTML 标签。
- 如果确实要展示 HTML 标签文本，应将其放在行内代码或明确标注语言的代码块中；如果要编写真正的 HTML，则必须保证标签结构完整且合法。

## 3. 本地提交前检查

修改任何教材 Markdown、代码示例、实验目录或文档构建配置后，在仓库根目录执行：

```bash
./scripts/check-book.sh
```

脚本会运行 `mdbook build book`，并检查构建输出中的 HTML 标签解析警告。出现未闭合 HTML 标签警告时，脚本会以非零状态退出；修复警告后再提交。

如果本机尚未安装 mdBook，可先执行：

```bash
cargo install mdbook
```

## 4. Pull Request 检查

GitHub Actions 会运行同一个 `scripts/check-book.sh`，并继续执行 Rust 实验的测试与 target check。CI 失败或出现 HTML 标签解析警告时，不应合并。

## 5. 内容准确性

- 区分可运行代码、伪代码、历史授课片段和按历史设计重构的代码。
- 只有实际执行过的实验才能标注为“已验证”；CI 编译通过不等于 QEMU 运行验证。
- 修改代码示例时，应同步检查依赖版本、导入、unsafe 契约和实验说明。
