use crate::{
    pmm::{PAGE_SIZE, Page, Pmm},
    uart::Uart,
};
use core::fmt::Write;

const HEADER_SIZE: u64 = 8;

struct Header {
    size: u64,
    is_occupied: bool,
}
impl Header {
    fn free(size: u64) -> Self {
        Header {
            size,
            is_occupied: false,
        }
    }
    fn occupied(size: u64) -> Self {
        Header {
            size,
            is_occupied: true,
        }
    }

    fn encode(&self) -> u64 {
        // This assumes out block size will never be higher than 2^63, so we can use first bit
        // to store the info.
        if self.is_occupied {
            self.size | 1 << 63
        } else {
            self.size & !(1 << 63)
        }
    }
    fn decode(raw: u64) -> Self {
        Header {
            size: raw & !(1 << 63),
            is_occupied: (raw >> 63) & 1 == 1,
        }
    }

    fn total_size(&self) -> u64 {
        HEADER_SIZE + self.size
    }

    unsafe fn from_ptr(block_ptr: *const u64) -> Header {
        let raw_header: u64 = unsafe { *(*block_ptr as *const u64) };
        Header::decode(raw_header)
    }
}

pub struct Heap {
    allocated_pages: usize,
    pages_ptr: *mut Page,
}
impl Heap {
    pub fn new(pmm: &mut Pmm) -> Heap {
        let page = pmm.alloc();
        Heap {
            allocated_pages: 0,
            pages_ptr: page.start_ptr as *mut Page,
        }
    }

    pub fn malloc(&mut self, pmm: &mut Pmm, size: u64) -> *mut u8 {
        let block_size = HEADER_SIZE + size;

        if block_size > PAGE_SIZE {
            let _ = writeln!(Uart, "Cannot allocate {size} bytes, block too big");
            panic!("block size too big");
        }

        let fit_ptr = self.first_page_with_fit(pmm, size);

        // Split block into two chunks, if the block size allows that.
        let header = unsafe { Header::from_ptr(fit_ptr as *const u64) };
        if header.size > size + HEADER_SIZE {
            let split_header = Header::free(header.size - size - HEADER_SIZE);
            unsafe {
                let split_ptr = fit_ptr.add(block_size as usize) as *mut u64;
                *split_ptr = split_header.encode();
            }
        }

        let header = Header::occupied(size);
        let header_ptr = fit_ptr as *mut u64;

        unsafe {
            *header_ptr = header.encode();
            fit_ptr.add(HEADER_SIZE as usize)
        }
    }

    fn first_page_with_fit(&mut self, pmm: &mut Pmm, size: u64) -> *mut u8 {
        assert!(HEADER_SIZE + size <= PAGE_SIZE);

        for i in 0..self.allocated_pages {
            let page = unsafe { &*self.pages_ptr.add(i) };
            if let Some(page_fit_ptr) = Heap::first_fit(page, size) {
                return page_fit_ptr;
            }
        }

        let page = Heap::request_page(pmm);
        let fit_ptr = Heap::first_fit(&page, size).unwrap();
        unsafe {
            let new_page_ptr = self.pages_ptr.add(self.allocated_pages);
            *new_page_ptr = page;
        }
        self.allocated_pages += 1;
        fit_ptr
    }

    fn first_fit(Page { start_ptr, .. }: &Page, size: u64) -> Option<*mut u8> {
        let mut block_ptr = *start_ptr;

        while unsafe { block_ptr.offset_from(*start_ptr) as u64 } < PAGE_SIZE - 1 {
            let header = unsafe { Header::from_ptr(block_ptr as *const u64) };

            if !header.is_occupied && header.size >= size {
                return Some(block_ptr);
            }

            unsafe {
                block_ptr = block_ptr.add(header.size as usize);
            }
        }

        None
    }

    fn request_page(pmm: &mut Pmm) -> Page {
        let page = pmm.alloc();

        // Create initial free block on a page, that spans the whole page.
        let header = Header::free(PAGE_SIZE - HEADER_SIZE);
        unsafe {
            let block_ptr = page.start_ptr as *mut u64;
            *block_ptr = header.encode();
        }

        page
    }

    pub fn free(&mut self, loc: *mut u64) {
        let block_ptr = unsafe { loc.sub(HEADER_SIZE as usize) };
        let header = unsafe { Header::from_ptr(block_ptr) };

        let next_header = unsafe { Header::from_ptr(block_ptr.add(header.total_size() as usize)) };
        let header = Header::free(if next_header.is_occupied {
            header.size
        } else {
            header.size + next_header.total_size()
        });

        unsafe {
            *block_ptr = header.encode();
        }
    }
}
