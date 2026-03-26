unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
}

const SIZE: u64 = 128 * 1024 * 1024;
const PAGE_SIZE: u64 = 4096;

struct Page {
    index: u64,
    start: u64,
}

struct Pmm {
    bitmap: *mut u64,
    len: u64,
}
impl Pmm {
    pub fn new() -> Pmm {
        let bitmap_start = (&raw const _kernel_end as u64) as *mut u64;
        Pmm {
            bitmap: bitmap_start,
            len: SIZE / PAGE_SIZE / 64,
        }
    }

    pub fn alloc(&mut self) -> Page {
        for n in 0..self.len {
            for i in 0..64 {
                let sector_ptr = unsafe { self.bitmap.add(n as usize) };
                let sector = unsafe { *sector_ptr };

                if (sector >> i) & 0b1 == 0 {
                    let index = (n * 64) + i;
                    let kernel_end = &raw const _kernel_end as u64;
                    let start = kernel_end + index * PAGE_SIZE;

                    if start > kernel_end + SIZE {
                        panic!("No free pages");
                    }

                    unsafe { *sector_ptr = sector | (0b1 << i) };
                    return Page { index, start };
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
