use core::ptr::{read_volatile, write_volatile};

use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{
    FrameAllocator,
    FrameDeallocator,
    PhysFrame,
    Size4KiB,
};

const PAGE_SIZE: u64 = 4096;

/// A single-core intrusive free-list allocator for 4 KiB physical frames.
///
/// The first 8 bytes of each free frame store the next frame's physical address.
/// free_head, physical_memory_offset, and free_frames are allocator metadata.
pub struct FreeListFrameAllocator {
    free_head: Option<PhysAddr>,
    physical_memory_offset: VirtAddr,
    free_frames: usize,
}

impl FreeListFrameAllocator {
    /// # Safety
    /// physical_memory_offset must map every physical frame that can be inserted.
    pub const unsafe fn new(physical_memory_offset: VirtAddr) -> Self {
        Self {
            free_head: None,
            physical_memory_offset,
            free_frames: 0,
        }
    }

    fn link_pointer(&self, frame: PhysFrame<Size4KiB>) -> *mut u64 {
        let virtual_address =
            self.physical_memory_offset + frame.start_address().as_u64();
        virtual_address.as_mut_ptr::<u64>()
    }

    pub fn free_frames(&self) -> usize {
        self.free_frames
    }

    /// Add all page-aligned frames from Usable memory regions.
    ///
    /// # Safety
    /// - The memory map must be the original Bootloader memory map.
    /// - The regions must not have been added before.
    /// - The direct map must cover the complete usable regions.
    /// - The caller must ensure no frame in those regions is already in use.
    pub unsafe fn add_usable_regions(&mut self, regions: &[MemoryRegion]) {
        for region in regions {
            if region.kind != MemoryRegionKind::Usable {
                continue;
            }

            let Some(start) = region.start.checked_add(PAGE_SIZE - 1) else {
                continue;
            };
            let start = start & !(PAGE_SIZE - 1);
            let end = region.end & !(PAGE_SIZE - 1);

            let mut address = start;
            while address < end {
                // Physical frame zero is reserved because zero is the list terminator.
                if address != 0 {
                    let frame = PhysFrame::from_start_address(PhysAddr::new(address))
                        .expect("usable region was page aligned");

                    // SAFETY: guaranteed by add_usable_regions' caller contract.
                    unsafe { self.deallocate_frame(frame) };
                }

                address += PAGE_SIZE;
            }
        }
    }
}

// SAFETY: the free list returns each node once, provided the caller never
// initializes a region twice or deallocates a frame twice.
unsafe impl FrameAllocator<Size4KiB> for FreeListFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let head = self.free_head?;
        let frame = PhysFrame::from_start_address(head).ok()?;

        // Read the next physical address stored in the first 8 bytes of this frame.
        // SAFETY: the frame is on the free list and the direct map covers it.
        let next = unsafe { read_volatile(self.link_pointer(frame)) };

        self.free_head = if next == 0 {
            None
        } else {
            Some(PhysAddr::new(next))
        };

        self.free_frames -= 1;
        Some(frame)
    }
}

impl FrameDeallocator<Size4KiB> for FreeListFrameAllocator {
    /// # Safety
    /// frame must be unused, belong to this allocator, not be mapped or shared,
    /// and must not already be present in the free list.
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let old_head = self.free_head
            .map(|address| address.as_u64())
            .unwrap_or(0);

        // Store the old head in the newly freed frame.
        // SAFETY: required by the method's contract.
        unsafe { write_volatile(self.link_pointer(frame), old_head) };

        self.free_head = Some(frame.start_address());
        self.free_frames += 1;
    }
}
