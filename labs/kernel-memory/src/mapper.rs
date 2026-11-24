use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        mapper::{MapToError, UnmapError},
        FrameAllocator,
        Mapper,
        OffsetPageTable,
        Page,
        PageTable,
        PageTableFlags,
        PhysFrame,
        Size4KiB,
    },
    VirtAddr,
};

/// Get the currently active level-4 page table through the physical direct map.
///
/// # Safety
/// - physical_memory_offset must be the correct complete physical direct-map offset.
/// - CR3 must contain a valid four-level page-table root.
/// - The caller must ensure that no other mutable reference to this table exists.
pub unsafe fn active_level_4_table(
    physical_memory_offset: VirtAddr,
) -> &'static mut PageTable {
    let (level_4_frame, _) = Cr3::read();
    let virtual_address =
        physical_memory_offset + level_4_frame.start_address().as_u64();

    // SAFETY: guaranteed by the function's caller.
    unsafe { &mut *virtual_address.as_mut_ptr::<PageTable>() }
}

/// Construct an OffsetPageTable for the active CR3 root.
///
/// # Safety
/// Same requirements as active_level_4_table. Call this only once for a root
/// table and keep the returned mapper under one owner.
pub unsafe fn init_offset_page_table(
    physical_memory_offset: VirtAddr,
) -> OffsetPageTable<'static> {
    let level_4_table =
        unsafe { active_level_4_table(physical_memory_offset) };

    // SAFETY: both the PML4 reference and direct-map offset are valid.
    unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

/// Map one 4 KiB virtual page to an already allocated physical frame.
///
/// # Safety
/// The caller must ensure the virtual page is unused, the frame is uniquely owned,
/// and the selected flags do not violate memory safety.
pub unsafe fn map_4k<A>(
    mapper: &mut OffsetPageTable<'_>,
    allocator: &mut A,
    virtual_address: VirtAddr,
    frame: PhysFrame<Size4KiB>,
    flags: PageTableFlags,
) -> Result<(), MapToError<Size4KiB>>
where
    A: FrameAllocator<Size4KiB>,
{
    let page = Page::containing_address(virtual_address);

    // SAFETY: mapping invariants are delegated to the caller.
    let flush = unsafe { mapper.map_to(page, frame, flags, allocator)? };
    flush.flush();

    Ok(())
}

/// Remove a 4 KiB mapping and return the old physical frame.
///
/// This function intentionally does not deallocate the frame. The caller must
/// decide whether the frame is exclusively owned or shared by other mappings.
pub fn unmap_4k(
    mapper: &mut OffsetPageTable<'_>,
    virtual_address: VirtAddr,
) -> Result<PhysFrame<Size4KiB>, UnmapError> {
    let page = Page::containing_address(virtual_address);
    let (frame, flush) = mapper.unmap(page)?;
    flush.flush();
    Ok(frame)
}
