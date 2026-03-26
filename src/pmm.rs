const START: u64 = 0x80000000;
const SIZE: u64 = 128 * 1024 * 1024;
const PAGE_SIZE: u64 = 4096;

struct Page {
    index: u64,
    start: u64,
}

struct Pmm<'a> {
    bitmap: &'a mut [u64],
}
impl<'a> Pmm<'a> {
    pub fn first(&mut self) -> Page {
        for (n, sector) in self.bitmap.iter_mut().enumerate() {
            for i in 0..64 {
                if (*sector >> i) & 0b1 == 0 {
                    let index = (n as u64 * 64) + i;
                    let start = START + (index) * PAGE_SIZE;

                    if start > START + SIZE {
                        panic!("No free pages");
                    }

                    *sector |= 0b1 << i;
                    return Page { index, start };
                }
            }
        }

        panic!("No free pages");
    }

    pub fn free(&mut self, index: u64) {
        let n = index / 64;
        let i = index % 64;

        *self.bitmap.get_mut(n as usize).unwrap() ^= 0b1 << i;
    }
}
