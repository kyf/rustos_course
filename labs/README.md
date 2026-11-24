# RustOS Labs

本目录用于保存与教程章节对应、可以重复构建和验证的实验代码。

目前整理版本以教材为主，历史对话中的部分代码片段和实验工程没有完整归档到当前目录。因此这里不会放入未经核验的伪代码，也不会声称这些实验已经能独立构建。后续每完成一项实验，再将实际验证过的代码、依赖版本、命令和输出整理进来。

建议结构：

```text
labs/
├── 02-cpu/
├── 03-boot/
├── 07-frame-allocator/
├── 08-page-mapping/
├── 11-virtual-address-allocator/
├── 13-address-space/
└── 16-syscall/
```

每个实验目录建议包含 `README.md`、源码、`expected-output.txt` 和 `notes.md`。
