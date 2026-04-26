use core::fmt::Write;

use crate::memory_manager::{MemoryManager, PAGE_SIZE};

#[repr(C)]
#[derive(Copy, Clone)]
struct Sector(u64);
impl Sector {
    const CAPACITY: usize = size_of::<Self>() * 8;

    fn is_full(&self) -> bool {
        self.0 == (!0)
    }

    fn free_page_index(&self) -> Option<usize> {
        (!self.is_full())
            .then(|| (0..Sector::CAPACITY).find(|i| (self.0 >> i) & 0b1 == 0))
            .flatten()
    }

    fn set_page_status(self, index: usize, is_occupied: bool) -> Sector {
        Self(if is_occupied {
            self.0 | (0b1 << index)
        } else {
            self.0 & !(0b1 << index)
        })
    }

    fn occupied_pages(&self) -> usize {
        (0..Sector::CAPACITY)
            .filter(|i| (self.0 >> i) & 0b1 != 0)
            .count()
    }
}

#[derive(Copy, Clone)]
struct Bitmap {
    ptr: *mut Sector,
    total_pages: usize,
}
impl Bitmap {
    fn sector(&self, n: usize) -> Sector {
        unsafe { *self.ptr.add(n) }
    }

    fn free_sector_index(&self) -> Option<usize> {
        let sectors = self.total_pages.div_ceil(Sector::CAPACITY);
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

        let page_index = (free_sector_index * Sector::CAPACITY) + sector_free_page_index;
        (page_index < self.total_pages).then_some(page_index)
    }

    fn set_page_status(&self, index: usize, is_occupied: bool) {
        assert!(
            index <= self.total_pages,
            "Cannot modify nonexistent page status"
        );

        let sector_index = index / Sector::CAPACITY;
        let in_sector_index = index % Sector::CAPACITY;

        unsafe {
            let sector_ptr = self.ptr.add(sector_index);
            *sector_ptr = (*sector_ptr).set_page_status(in_sector_index, is_occupied);
        }
    }
}
impl core::fmt::Display for Bitmap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sectors = self.total_pages.div_ceil(Sector::CAPACITY);
        let occupied_pages = (0..sectors).try_fold(0, |acc, sector_index| {
            let sector = self.sector(sector_index);
            writeln!(f, "{:064b}", sector.0)?;
            Ok(acc + sector.occupied_pages())
        })?;
        writeln!(f, "{} / {} pages", occupied_pages, self.total_pages)?;
        writeln!(f, "{sectors} sectors")
    }
}

#[derive(Copy, Clone)]
pub struct Pmm {
    memory_start_ptr: *const u8,
    bitmap: Bitmap,
}
impl MemoryManager for Pmm {
    fn alloc(&mut self) -> Option<*const ()> {
        let free_page_index = self.bitmap.free_page_index()?;

        self.bitmap.set_page_status(free_page_index, true);

        let page_ptr =
            unsafe { self.memory_start_ptr.add(free_page_index * PAGE_SIZE) as *const () };
        Some(page_ptr)
    }

    fn free(&mut self, page_ptr: *const ()) {
        let page_loc = page_ptr as usize;
        let memory_start_loc = self.memory_start_ptr as usize;
        let relative_page_offset = page_loc - memory_start_loc;
        let page_index = relative_page_offset / PAGE_SIZE;
        self.bitmap.set_page_status(page_index, false);
    }
}

// This assumes memory size is specified in bytes, and free_memory_page_start_ptr is a pointer
// to the first location in ram that is free, and aligned to PAGE_SIZE, also all following
// addresses must be empty.
pub fn init_pmm(memory_size: usize) -> Pmm {
    unsafe extern "C" {
        static _memory_start_virt: u8;
        static _free_memory_start_virt: u8;
    }

    const PAGE_SIZE: usize = 4096; // TODO: This should be defined in the linker file.

    let memory_start_ptr = unsafe { &_memory_start_virt } as *const u8;
    let free_memory_page_start_ptr = unsafe { &_free_memory_start_virt } as *const u8;

    let occupied_locations = free_memory_page_start_ptr as usize - memory_start_ptr as usize;
    let occupied_pages = occupied_locations / PAGE_SIZE;

    // Move by one page, because the first page is occupied by the root page table.
    let bitmap_start_ptr = unsafe { free_memory_page_start_ptr.add(PAGE_SIZE) };
    let total_pages = memory_size / PAGE_SIZE;
    let bitmap = init_bitmap(bitmap_start_ptr as *mut u8, occupied_pages, total_pages);

    Pmm {
        bitmap,
        memory_start_ptr: free_memory_page_start_ptr,
    }
}

fn init_bitmap(ptr: *mut u8, preoccupied_pages: usize, total_pages: usize) -> Bitmap {
    let ptr = ptr as *mut Sector;

    // Ensure the memory is clean before writing the bitmap.
    for i in 0..total_pages / Sector::CAPACITY {
        unsafe { *ptr.add(i) = Sector(0) };
    }

    // Reserve the pages needed to store the bitmap itself.
    let bits_on_page = PAGE_SIZE * 8;
    let bitmap_occupied_pages = total_pages.div_ceil(bits_on_page);
    let total_occupied_pages = preoccupied_pages + bitmap_occupied_pages;
    let bitmap_fully_occupied_sectors = total_occupied_pages / Sector::CAPACITY;
    for i in 0..bitmap_fully_occupied_sectors {
        unsafe { *ptr.add(i) = Sector(!0) };
    }
    let leftover_bits = total_occupied_pages % Sector::CAPACITY;
    if leftover_bits != 0 {
        let mut sector_mask = 0;
        for _ in 0..leftover_bits {
            sector_mask = (sector_mask << 1) + 1;
        }
        unsafe { *ptr.add(bitmap_fully_occupied_sectors) = Sector(sector_mask) };
    }

    Bitmap { ptr, total_pages }
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
            bitmap: Bitmap::init_bitmap(bitmap_ptr as *mut u8, 0, 4),
            memory_start_ptr: mem.0.as_ptr(),
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
