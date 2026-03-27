unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
}

const SIZE: u64 = 128 * 1024 * 1024;
pub const PAGE_SIZE: u64 = 4096;

pub struct Page {
    pub index: u64,
    pub start_ptr: *mut u8,
}

pub struct Pmm {
    pub bitmap: *mut u64,
    len: u64,
}
impl Pmm {
    pub fn new() -> Pmm {
        let kernel_start = &raw const _kernel_start as u64;
        let kernel_end = &raw const _kernel_end as u64;
        let kernel_occupied_pages = (kernel_end - kernel_start) / PAGE_SIZE + 1;

        let bitmap_start = (kernel_end + 7) & !7;
        let bitmap_start_ptr = bitmap_start as *mut u64;
        let bitmap_size = SIZE / PAGE_SIZE / 64;
        let bitmap_size_bytes = SIZE / PAGE_SIZE / 8;
        let bitmap_occupied_pages = bitmap_size_bytes / PAGE_SIZE + 1;

        let mut alloc_mask = 0;
        for _ in 0..=(kernel_occupied_pages + bitmap_occupied_pages) {
            alloc_mask = (alloc_mask << 1) | 0b1;
        }

        unsafe { *bitmap_start_ptr = alloc_mask };

        Pmm {
            bitmap: bitmap_start_ptr,
            len: bitmap_size,
        }
    }

    pub fn alloc(&mut self) -> Page {
        let kernel_end = &raw const _kernel_end as u64;

        for n in 0..self.len {
            for i in 0..64 {
                let sector_ptr = unsafe { self.bitmap.add(n as usize) };
                let sector = unsafe { *sector_ptr };

                if (sector >> i) & 0b1 == 0 {
                    let index = (n * 64) + i;
                    let start = kernel_end + index * PAGE_SIZE;

                    if start > kernel_end + SIZE {
                        panic!("No free pages");
                    }

                    unsafe { *sector_ptr = sector | (0b1 << i) };
                    return Page {
                        index,
                        start_ptr: start as *mut u8,
                    };
                }
            }
        }

        panic!("No free pages");
    }

    pub fn free(&mut self, index: u64) {
        let n = index / 64;
        let i = index % 64;

        let sector_ptr = unsafe { self.bitmap.add(n as usize) };
        let sector = unsafe { *sector_ptr };
        unsafe { *sector_ptr = sector & !(0b1 << i) };
    }
}
