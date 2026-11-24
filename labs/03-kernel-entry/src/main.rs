#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // BootInfo 是 Bootloader 传入的启动环境信息。
    // 这里只读取 MemoryRegion 数量，不进行物理页分配。
    let _memory_region_count = boot_info.memory_regions.len();

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
