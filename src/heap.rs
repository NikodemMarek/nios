use crate::{
    pmm::{PAGE_SIZE, Page, Pmm},
    uart::Uart,
};
use core::fmt::Write;

const HEADER_SIZE: u64 = 8;

struct Block {
    capacity: u64,
    is_occupied: bool,
}
impl Block {
    fn free(size: u64) -> Self {
        Block {
            capacity: size,
            is_occupied: false,
        }
    }
    fn occupied(size: u64) -> Self {
        Block {
            capacity: size,
            is_occupied: true,
        }
    }

    fn encode(&self) -> u64 {
        // This assumes out block size will never be higher than 2^63, so we can use first bit
        // to store the info.
        if self.is_occupied {
            self.capacity | 1 << 63
        } else {
            self.capacity & !(1 << 63)
        }
    }
    fn decode(raw: u64) -> Self {
        Block {
            capacity: raw & !(1 << 63),
            is_occupied: (raw >> 63) & 1 == 1,
        }
    }

    fn size(&self) -> u64 {
        HEADER_SIZE + self.capacity
    }

    unsafe fn from_ptr(block_ptr: *const u8) -> Block {
        let raw_header = unsafe { *(block_ptr as *const u64) };
        Block::decode(raw_header)
    }
}

pub struct Heap {
    pmm: Pmm,
    allocated_pages: usize,
    pages_ptr: *mut Page,
}
impl Heap {
    pub fn new(mut pmm: Pmm) -> Heap {
        let pages_page = pmm.alloc();
        let mut heap = Heap {
            pmm,
            allocated_pages: 0,
            pages_ptr: pages_page.start_ptr as *mut Page,
        };

        heap.new_page();
        heap
    }

    pub fn alloc_array<T: Sized>(&mut self, capacity: usize) -> &mut [T] {
        let size = capacity * size_of::<T>();
        let array_ptr = self.malloc(size as u64) as *mut T;
        unsafe { core::slice::from_raw_parts_mut(array_ptr, capacity) }
    }

    pub fn malloc(&mut self, size: u64) -> *const u8 {
        let block = Block::occupied(size);

        if block.size() > PAGE_SIZE {
            let _ = writeln!(Uart, "Cannot allocate {size} bytes, block too big");
            panic!("block size too big");
        }

        let fit_ptr = self.first_page_with_fit(size);

        // Split block into two chunks, if the block size allows that.
        let block = unsafe { Block::from_ptr(fit_ptr) };
        if block.capacity > size + HEADER_SIZE {
            let split_header = Block::free(block.capacity - size - HEADER_SIZE);
            unsafe {
                let split_ptr = fit_ptr.add(block.size() as usize) as *mut u64;
                *split_ptr = split_header.encode();
            }
        }

        let block = Block::occupied(size);
        let block_ptr = fit_ptr as *mut u64;

        unsafe {
            *block_ptr = block.encode();
            fit_ptr.add(HEADER_SIZE as usize)
        }
    }

    fn first_page_with_fit(&mut self, size: u64) -> *const u8 {
        assert!(HEADER_SIZE + size <= PAGE_SIZE);

        for i in 0..self.allocated_pages {
            let page = unsafe { &*self.pages_ptr.add(i) };
            if let Some(page_fit_ptr) = Heap::first_fit(page.start_ptr, size) {
                return page_fit_ptr;
            }
        }

        let new_page_ptr = self.new_page();
        Heap::first_fit(new_page_ptr, size)
            .expect("If block passed the assertion, it has to fit on an empty page")
    }

    fn first_fit(page_ptr: *const u8, size: u64) -> Option<*const u8> {
        let mut block_ptr = page_ptr;

        while unsafe { block_ptr.offset_from(page_ptr) as u64 } < PAGE_SIZE {
            let block = unsafe { Block::from_ptr(block_ptr) };

            if !block.is_occupied && block.capacity >= size {
                return Some(block_ptr);
            }

            unsafe {
                block_ptr = block_ptr.add(block.size() as usize);
            }
        }

        None
    }

    fn new_page(&mut self) -> *const u8 {
        let page = Heap::request_page(&mut self.pmm);
        let page_ptr_cp = page.start_ptr;
        unsafe {
            let page_ptr = self.pages_ptr.add(self.allocated_pages);
            *page_ptr = page;
        }
        self.allocated_pages += 1;
        page_ptr_cp
    }

    fn request_page(pmm: &mut Pmm) -> Page {
        let page = pmm.alloc();

        // Create initial free block on a page, that spans the whole page.
        let block = Block::free(PAGE_SIZE - HEADER_SIZE);
        unsafe {
            let block_ptr = page.start_ptr as *mut u64;
            *block_ptr = block.encode();
        }

        page
    }

    pub fn free(&mut self, loc: *mut u8) {
        let block_ptr = unsafe { loc.sub(HEADER_SIZE as usize) };
        let block = unsafe { Block::from_ptr(block_ptr) };

        let next_block = unsafe {
            let next_block_ptr = block_ptr.add(block.size() as usize);
            Block::from_ptr(next_block_ptr)
        };
        let block = Block::free(if next_block.is_occupied {
            block.capacity
        } else {
            block.capacity + next_block.size()
        });

        unsafe {
            let block_ptr = block_ptr as *mut u64;
            *block_ptr = block.encode();
        }
    }
}
