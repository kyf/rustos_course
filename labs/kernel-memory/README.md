# kernel-memory

这是供教学内核集成的 no_std 内存管理模块，不是完整内核，也不能单独启动 QEMU。

## 包含内容

- frame_allocator.rs：侵入式 Free List 物理页帧分配器。
- mapper.rs：从 CR3 取得当前 PML4、初始化 OffsetPageTable、映射和解除映射 4 KiB 页面。

## 依赖版本

~~~toml
bootloader_api = "=0.11.17"
x86_64 = { version = "=0.15.5", default-features = false, features = ["instructions"] }
~~~

## 检查

~~~bash
cargo check --manifest-path labs/kernel-memory/Cargo.toml \
  --target x86_64-unknown-none
~~~

## 集成前提

1. Bootloader 启用了完整物理内存直接映射。
2. BootInfo.physical_memory_offset 是 Some(offset)。
3. 只把 MemoryRegionKind::Usable 区域加入 Free List。
4. 不重复初始化相同区域，不重复释放页帧。
5. 解除映射与释放页帧是两个操作；共享页帧不能被当前映射的所有者擅自释放。
6. Mapper / FreeList 在多核环境中使用前必须增加同步。
