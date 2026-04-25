use core::fmt::Write;

use crate::memory_manager::{MemoryManager, PAGE_SIZE};

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

#[derive(Copy, Clone)]
struct Bitmap {
    ptr: *mut u64,
    total_pages: usize,
}
impl Bitmap {
    fn new(ptr: *mut u8, preoccupied_pages: usize, total_pages: usize) -> Self {
        let ptr = ptr as *mut u64;

        // Reserve the pages needed to store the bitmap itself.
        let bits_on_page = PAGE_SIZE * 8;
        let bitmap_occupied_pages = total_pages.div_ceil(bits_on_page);
        let total_occupied_pages = preoccupied_pages + bitmap_occupied_pages;
        let bitmap_fully_occupied_sectors = total_occupied_pages / Sector::capacity();
        for i in 0..bitmap_fully_occupied_sectors {
            unsafe {
                let sector_ptr = ptr.add(i);
                *sector_ptr = !0;
            };
        }
        let leftover_bits = total_occupied_pages % Sector::capacity();
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

        Self { ptr, total_pages }
    }

    fn sector(&self, n: usize) -> Sector {
        let bitmap_sector_ptr = unsafe { self.ptr.add(n) };
        let sector_bitmap = unsafe { *bitmap_sector_ptr };
        Sector(sector_bitmap)
    }

    fn free_sector_index(&self) -> Option<usize> {
        let sectors = self.total_pages.div_ceil(Sector::capacity());
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
        (page_index < self.total_pages).then_some(page_index)
    }

    fn set_page_status(&self, index: usize, is_occupied: bool) {
        assert!(
            index <= self.total_pages,
            "Cannot modify nonexistent page status"
        );

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
        let sectors = self.total_pages.div_ceil(Sector::capacity());
        for sector_index in 0..sectors {
            let sector = self.sector(sector_index);
            writeln!(f, "{:064b}", sector.0)?;
        }
        writeln!(f, "{} pages", self.total_pages)?;
        writeln!(f, "{sectors} sectors")?;

        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct Pmm {
    ptr: *const u8,
    bitmap: Bitmap,
}
impl Pmm {
    pub fn init() -> Self {
        let kernel_end_ptr = unsafe { &super::_kernel_end } as *const u8;
        let kernel_end_loc = kernel_end_ptr as usize;

        let page_after_kernel_loc = (kernel_end_loc + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // let memory_start_ptr = unsafe { &super::_memory_start } as *const u8;
        let memory_start_ptr = 0x0 as *const u8;
        let memory_end_ptr = unsafe { &super::_memory_end } as *const u8;
        let memory_size = unsafe { memory_end_ptr.offset_from(memory_start_ptr) as usize } + 1;

        let occupied_memory_size =
            unsafe { kernel_end_ptr.offset_from(memory_start_ptr) as usize } + 1;
        let occupied_pages = occupied_memory_size.div_ceil(PAGE_SIZE);

        let bitmap_start_loc = page_after_kernel_loc;
        let total_pages = memory_size / PAGE_SIZE;
        let bitmap = Bitmap::new(bitmap_start_loc as *mut u8, occupied_pages, total_pages);

        Self {
            bitmap,
            ptr: memory_start_ptr,
        }
    }

    pub fn to_raw(&self) -> (*const (), usize) {
        (self.bitmap.ptr as *const (), self.bitmap.total_pages)
    }
    pub fn from_raw(
        memory_start_ptr: *const (),
        bitmap_ptr: *const (),
        total_pages: usize,
    ) -> Self {
        let bitmap = Bitmap {
            ptr: bitmap_ptr as *mut u64,
            total_pages,
        };
        Self {
            ptr: memory_start_ptr as *const u8,
            bitmap,
        }
    }
}

impl MemoryManager for Pmm {
    fn alloc(&mut self) -> Option<*const ()> {
        let free_page_index = self.bitmap.free_page_index()?;

        self.bitmap.set_page_status(free_page_index, true);

        // This is test workaround, ideally PMM should work with any memory pointer.
        let page_ptr = if cfg!(test) {
            unsafe { self.ptr.add(free_page_index * PAGE_SIZE) as *const () }
        } else {
            (free_page_index * PAGE_SIZE) as *const ()
        };
        Some(page_ptr)
    }

    fn free(&mut self, page_ptr: *const ()) {
        let page_loc = page_ptr as usize;
        let memory_start_loc = self.ptr as usize;
        let relative_page_offset = page_loc - memory_start_loc;
        let page_index = relative_page_offset / PAGE_SIZE;
        self.bitmap.set_page_status(page_index, false);
    }
}

// This assumes memory size is specified in bytes, and free_memory_page_start_ptr is a pointer
// to the first location in ram that is free, and aligned to PAGE_SIZE, also all following
// addresses must be empty.
#[unsafe(link_section = ".text.boot")]
#[unsafe(no_mangle)]
pub extern "C" fn init_bitmap(
    memory_size: usize,
    memory_start_ptr: *const u8,
    free_memory_page_start_ptr: *const u8,
) -> usize {
    const PAGE_SIZE: usize = 4096; // TOOD: This should be defined in the linker file.

    let occupied_locations = free_memory_page_start_ptr as usize - memory_start_ptr as usize;
    let occupied_pages = occupied_locations / PAGE_SIZE;

    let bitmap_start_loc = free_memory_page_start_ptr as usize;
    let total_pages = memory_size / PAGE_SIZE;

    fn write_bitmap(bitmap_start_ptr: *mut u8, preoccupied_pages: usize, total_pages: usize) {
        let ptr = bitmap_start_ptr as *mut u64;

        // Reserve the pages needed to store the bitmap itself.
        let bits_on_page = PAGE_SIZE * 8;
        let bitmap_occupied_pages = total_pages.div_ceil(bits_on_page);
        let total_occupied_pages = preoccupied_pages + bitmap_occupied_pages;
        let bitmap_fully_occupied_sectors = total_occupied_pages / Sector::capacity();
        for i in 0..bitmap_fully_occupied_sectors {
            unsafe {
                let sector_ptr = ptr.add(i);
                *sector_ptr = !0;
            };
        }
        let leftover_bits = total_occupied_pages % Sector::capacity();
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
    }
    write_bitmap(bitmap_start_loc as *mut u8, occupied_pages, total_pages);

    total_pages
}

#[unsafe(link_section = ".text.boot")]
#[unsafe(no_mangle)]
pub extern "C" fn print_bitmap(bitmap_ptr: *const u8, total_pages: usize) {
    let bitmap = Bitmap {
        ptr: bitmap_ptr as *mut u64,
        total_pages,
    };
    writeln!(crate::sbi::Sbi, "{}", bitmap);
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
            bitmap: Bitmap::new(bitmap_ptr as *mut u8, 0, 4),
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
