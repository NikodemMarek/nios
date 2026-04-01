use crate::memory_manager::{MemoryManager, PAGE_SIZE};

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
    static _memory_end: u8;
}

struct Sector(u64);
impl Sector {
    const fn capacity() -> usize {
        64
    }

    fn is_full(&self) -> bool {
        self.0 == (!0)
    }

    fn free_page_index(&self) -> Option<usize> {
        (!self.is_full())
            .then(|| (0..Sector::capacity()).find(|i| (self.0 >> i) & 0b1 == 0))
            .flatten()
    }
}

struct Bitmap {
    ptr: *mut u64,
    pages: usize,
}
impl Bitmap {
    fn new(ptr: *mut u8, pages: usize) -> Self {
        let ptr = ptr as *mut u64;

        // Reserve the pages needed to store the bitmap itself.
        let bits_on_page = PAGE_SIZE * 8;
        let pages_occupied_by_bitmap = pages.div_ceil(bits_on_page);
        let bitmap_fully_occupied_sectors = pages_occupied_by_bitmap / Sector::capacity();
        for i in 0..bitmap_fully_occupied_sectors {
            unsafe {
                let sector_ptr = ptr.add(i);
                *sector_ptr = !0;
            };
        }
        let leftover_bits = pages_occupied_by_bitmap % Sector::capacity();
        if leftover_bits != 0 {
            let mut sector_mask = 0;
            for _ in 0..leftover_bits {
                sector_mask = (sector_mask << 1) + 1;
            }
            unsafe {
                let leftover_sector_ptr = ptr.add(bitmap_fully_occupied_sectors);
                *leftover_sector_ptr = sector_mask;
            };
        }

        Self { ptr, pages }
    }

    fn sector(&self, n: usize) -> Sector {
        let bitmap_sector_ptr = unsafe { self.ptr.add(n) };
        let sector_bitmap = unsafe { *bitmap_sector_ptr };
        Sector(sector_bitmap)
    }

    fn free_sector_index(&self) -> Option<usize> {
        let sectors = self.pages.div_ceil(Sector::capacity());
        for sector_index in 0..sectors {
            let sector = self.sector(sector_index);
            if !sector.is_full() {
                return Some(sector_index);
            }
        }
        None
    }

    fn free_page_index(&self) -> Option<usize> {
        let free_sector_index = self.free_sector_index()?;
        let sector = self.sector(free_sector_index);
        let sector_free_page_index = sector.free_page_index()?;

        let page_index = (free_sector_index * Sector::capacity()) + sector_free_page_index;
        (page_index < self.pages).then_some(page_index)
    }

    fn set_page_status(&self, index: usize, is_occupied: bool) {
        assert!(index <= self.pages, "Cannot modify nonexistent page status");

        let n = index / Sector::capacity();
        let i = index % Sector::capacity();

        let sector_ptr = unsafe { self.ptr.add(n) };
        let sector_bitmap = unsafe { *sector_ptr };
        if is_occupied {
            unsafe { *sector_ptr = sector_bitmap | (0b1 << i) };
        } else {
            unsafe { *sector_ptr = sector_bitmap & !(0b1 << i) };
        }
    }
}
impl core::fmt::Display for Bitmap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sectors = self.pages.div_ceil(Sector::capacity());
        for sector_index in 0..sectors {
            let sector = self.sector(sector_index);
            writeln!(f, "{:064b}", sector.0)?;
        }
        writeln!(f, "{} pages", self.pages)?;
        writeln!(f, "{sectors} sectors")?;

        Ok(())
    }
}

fn calculate_free_memory() -> (*const u8, usize) {
    let kernel_end_ptr = unsafe { &_kernel_end } as *const u8;
    let memory_end_ptr = unsafe { &_memory_end } as *const u8;

    let free_memory_start_ptr = unsafe { kernel_end_ptr.add(1) };
    let free_memory_size = unsafe { memory_end_ptr.offset_from(kernel_end_ptr) as usize } + 1;

    (free_memory_start_ptr, free_memory_size)
}

pub struct Pmm {
    ptr: *const u8,
    bitmap: Bitmap,
}
impl Pmm {
    pub fn init() -> Self {
        let (free_memory_start_ptr, memory_size) = calculate_free_memory();

        let free_memory_start_loc = free_memory_start_ptr as usize;
        let free_memory_start_aligned_loc =
            (free_memory_start_loc + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let free_memory_start_aligned_ptr = free_memory_start_aligned_loc as *const u8;

        let free_memory_alignment_offset = free_memory_start_aligned_loc - free_memory_start_loc;
        let effective_memory_size = memory_size - free_memory_alignment_offset;

        Self::new(free_memory_start_aligned_ptr, effective_memory_size)
    }

    fn new(free_memory_ptr: *const u8, free_memory_size: usize) -> Self {
        let free_pages = free_memory_size / PAGE_SIZE;
        Self {
            bitmap: Bitmap::new(free_memory_ptr as *mut u8, free_pages),
            ptr: free_memory_ptr,
        }
    }
}

impl MemoryManager for Pmm {
    fn alloc(&mut self) -> Option<*const u8> {
        let free_page_index = self.bitmap.free_page_index()?;

        self.bitmap.set_page_status(free_page_index, true);

        Some(unsafe { self.ptr.add(free_page_index * PAGE_SIZE) })
    }

    fn free(&mut self, page_ptr: *const u8) {
        let page_loc = page_ptr as usize;
        let memory_start_loc = self.ptr as usize;
        let relative_page_offset = page_loc - memory_start_loc;
        let page_index = relative_page_offset / PAGE_SIZE;
        self.bitmap.set_page_status(page_index, false);
    }
}

#[cfg(test)]
pub mod tests {
    use crate::memory_manager::MemoryManager;

    use super::{Bitmap, PAGE_SIZE, Pmm};

    pub fn setup_test_pmm() -> Pmm {
        #[repr(align(4096))]
        struct MockMemory([u8; PAGE_SIZE * 4]);
        let mem = MockMemory([0; PAGE_SIZE * 4]);

        let mut bitmap = [0u64; 1];
        let bitmap_ptr = bitmap.as_mut_ptr();
        Pmm {
            bitmap: Bitmap::new(bitmap_ptr as *mut u8, 4),
            ptr: mem.0.as_ptr(),
        }
    }

    #[test_case]
    fn test_alloc() {
        let mut pmm = setup_test_pmm();

        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b0001);
        pmm.alloc();
        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b0011);
        pmm.alloc();
        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b0111);
        pmm.alloc();
        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b1111);

        assert_eq!(pmm.alloc(), None);
    }

    #[test_case]
    fn test_free() {
        let mut pmm = setup_test_pmm();

        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b0001);
        let ptr1 = pmm.alloc().unwrap();
        pmm.alloc();
        let ptr2 = pmm.alloc().unwrap();
        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b1111);
        assert_eq!(pmm.alloc(), None);

        pmm.free(ptr2);
        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b0111);
        pmm.free(ptr1);
        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b0101);

        pmm.alloc();
        pmm.alloc();
        assert_eq!(unsafe { *pmm.bitmap.ptr }, 0b1111);
    }
}
