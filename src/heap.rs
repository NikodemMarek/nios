use crate::{
    pmm::{PAGE_SIZE, Page, Pmm},
    uart::Uart,
};
use core::fmt::Write;

const HEADER_SIZE: u64 = 8;

struct Header {
    block_size: u64,
    is_occupied: bool,
}
impl Header {
    fn free(size: u64) -> Self {
        Header {
            block_size: size,
            is_occupied: false,
        }
    }
    fn occupied(size: u64) -> Self {
        Header {
            block_size: size,
            is_occupied: true,
        }
    }

    fn encode(&self) -> u64 {
        // This assumes out block size will never be higher than 2^63, so we can use first bit
        // to store the info.
        if self.is_occupied {
            self.block_size | 1 << 63
        } else {
            self.block_size & !(1 << 63)
        }
    }
    fn decode(raw: u64) -> Self {
        Header {
            block_size: raw & !(1 << 63),
            is_occupied: (raw >> 63) & 1 == 1,
        }
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

        let fit_ptr = self.first_page_with_fit(pmm, block_size);
        let header = Header::occupied(block_size);
        let header_ptr = fit_ptr as *mut u64;

        unsafe {
            *header_ptr = header.encode();
            fit_ptr.add(HEADER_SIZE as usize)
        }
    }

    fn first_page_with_fit(&mut self, pmm: &mut Pmm, size: u64) -> *mut u8 {
        assert!(size <= PAGE_SIZE);

        for i in 0..self.allocated_pages {
            let page = unsafe { &*self.pages_ptr.add(i) };
            if let Some(page_fit_ptr) = Heap::first_fit(page, size) {
                return page_fit_ptr;
            }
        }

        let page = pmm.alloc();
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
            let raw_header: u64 = unsafe { *(*block_ptr as *const u64) };
            let header = Header::decode(raw_header);

            if !header.is_occupied && header.block_size >= size {
                return Some(block_ptr);
            }

            unsafe {
                block_ptr = block_ptr.add(header.block_size as usize);
            }
        }

        None
    }

    pub fn free(&mut self, loc: *mut u64) {
        let block_ptr = unsafe { loc.sub(HEADER_SIZE as usize) };
        let raw_header: u64 = unsafe { *(*block_ptr as *const u64) };
        let header = Header::decode(raw_header);

        let header = Header::free(header.block_size);
        unsafe {
            *block_ptr = header.encode();
        }
    }
}
