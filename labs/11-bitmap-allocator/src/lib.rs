#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeError {
    InvalidRange,
    DoubleFree,
}

pub struct BitmapAllocator<'a> {
    words: &'a mut [u64],
    page_count: usize,
}

impl<'a> BitmapAllocator<'a> {
    pub fn new(words: &'a mut [u64], page_count: usize) -> Option<Self> {
        let required_words = page_count.checked_add(63)? / 64;
        if words.len() < required_words {
            return None;
        }

        words.fill(0);
        Some(Self { words, page_count })
    }

    fn is_used(&self, page: usize) -> bool {
        let word = page / 64;
        let bit = page % 64;
        (self.words[word] & (1u64 << bit)) != 0
    }

    fn set_used(&mut self, page: usize, used: bool) {
        let word = page / 64;
        let bit = page % 64;
        let mask = 1u64 << bit;

        if used {
            self.words[word] |= mask;
        } else {
            self.words[word] &= !mask;
        }
    }

    pub fn reserve(&mut self, start: usize, count: usize) -> Result<(), FreeError> {
        let end = start.checked_add(count).ok_or(FreeError::InvalidRange)?;
        if count == 0 || end > self.page_count {
            return Err(FreeError::InvalidRange);
        }
        if (start..end).any(|page| self.is_used(page)) {
            return Err(FreeError::InvalidRange);
        }
        for page in start..end {
            self.set_used(page, true);
        }
        Ok(())
    }

    pub fn allocate_first_fit(
        &mut self,
        count: usize,
        align_pages: usize,
    ) -> Option<usize> {
        if count == 0 || align_pages == 0 || count > self.page_count {
            return None;
        }

        let last_start = self.page_count - count;
        for start in 0..=last_start {
            if start % align_pages != 0 {
                continue;
            }

            let end = start + count;
            if (start..end).all(|page| !self.is_used(page)) {
                for page in start..end {
                    self.set_used(page, true);
                }
                return Some(start);
            }
        }
        None
    }

    pub fn deallocate(
        &mut self,
        start: usize,
        count: usize,
    ) -> Result<(), FreeError> {
        let end = start.checked_add(count).ok_or(FreeError::InvalidRange)?;
        if count == 0 || end > self.page_count {
            return Err(FreeError::InvalidRange);
        }
        if (start..end).any(|page| !self.is_used(page)) {
            return Err(FreeError::DoubleFree);
        }
        for page in start..end {
            self.set_used(page, false);
        }
        Ok(())
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::{BitmapAllocator, FreeError};

    #[test]
    fn first_fit_returns_first_contiguous_range() {
        let mut words = [0u64; 2];
        let mut allocator = BitmapAllocator::new(&mut words, 100).unwrap();

        assert_eq!(allocator.allocate_first_fit(3, 1), Some(0));
        assert_eq!(allocator.allocate_first_fit(2, 1), Some(3));
    }

    #[test]
    fn alignment_is_applied_to_start_page_index() {
        let mut words = [0u64; 2];
        let mut allocator = BitmapAllocator::new(&mut words, 100).unwrap();

        assert_eq!(allocator.allocate_first_fit(1, 1), Some(0));
        assert_eq!(allocator.allocate_first_fit(1, 4), Some(4));
    }

    #[test]
    fn deallocated_range_can_be_reused() {
        let mut words = [0u64; 2];
        let mut allocator = BitmapAllocator::new(&mut words, 100).unwrap();

        let start = allocator.allocate_first_fit(4, 1).unwrap();
        allocator.deallocate(start, 4).unwrap();

        assert_eq!(allocator.allocate_first_fit(4, 1), Some(0));
    }

    #[test]
    fn double_free_is_rejected() {
        let mut words = [0u64; 2];
        let mut allocator = BitmapAllocator::new(&mut words, 100).unwrap();

        let start = allocator.allocate_first_fit(2, 1).unwrap();
        allocator.deallocate(start, 2).unwrap();

        assert_eq!(allocator.deallocate(start, 2), Err(FreeError::DoubleFree));
    }

    #[test]
    fn reserved_pages_are_skipped() {
        let mut words = [0u64; 2];
        let mut allocator = BitmapAllocator::new(&mut words, 100).unwrap();

        allocator.reserve(0, 3).unwrap();
        assert_eq!(allocator.allocate_first_fit(2, 1), Some(3));
    }
}
