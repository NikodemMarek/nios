use crate::pmm::{PAGE_SIZE, Pmm};

const HEADER_SIZE: usize = 8;

struct Header {
    ptr: *const u8,
    capacity: usize,
    is_occupied: bool,
}
impl Header {
    fn new(ptr: *const u8, capacity: usize, is_occupied: bool) -> Self {
        Self {
            ptr,
            capacity,
            is_occupied,
        }
    }

    fn encode(&self) -> u64 {
        // This assumes out block size will never be higher than 2^63, so we can use first bit
        // to store the info.
        if self.is_occupied {
            self.capacity as u64 | 1 << 63
        } else {
            self.capacity as u64 & !(1 << 63)
        }
    }

    fn size(&self) -> usize {
        HEADER_SIZE + self.capacity
    }

    unsafe fn from_ptr(ptr: *const u8) -> Self {
        let raw_header = unsafe { *(ptr as *const u64) };
        Self {
            ptr,
            capacity: (raw_header & !(1 << 63)) as usize,
            is_occupied: (raw_header >> 63) & 1 == 1,
        }
    }

    unsafe fn write(&self) {
        unsafe {
            *(self.ptr as *mut u64) = self.encode();
        }
    }

    fn content_ptr(&self) -> *const u8 {
        unsafe { self.ptr.add(HEADER_SIZE) }
    }
}

struct Block {
    header: Header,
    alignment_offset: usize,
}
impl Block {
    fn new(header: Header, alignment_offset: usize) -> Self {
        Self {
            header,
            alignment_offset,
        }
    }

    unsafe fn write(&self) {
        unsafe {
            self.header.write();
            *(self.offset_ptr() as *mut u8) = self.alignment_offset as u8;
        }
    }

    fn offset_ptr(&self) -> *const u8 {
        unsafe { self.aligned_data_ptr().sub(1) }
    }

    fn aligned_data_ptr(&self) -> *const u8 {
        unsafe { self.header.content_ptr().add(self.alignment_offset) }
    }

    fn aligned_capacity(&self) -> usize {
        self.header.capacity - self.alignment_offset
    }

    #[inline]
    fn size(&self) -> usize {
        self.header.size()
    }

    unsafe fn from_aligned_data_ptr(aligned_data_ptr: *const u8) -> Self {
        let alignment_offset = unsafe {
            let offset_ptr = aligned_data_ptr.sub(1);
            *offset_ptr as usize
        };

        let block_header_ptr = unsafe { aligned_data_ptr.sub(HEADER_SIZE + alignment_offset) };

        Self {
            header: unsafe { Header::from_ptr(block_header_ptr) },
            alignment_offset,
        }
    }
}

pub struct Heap {
    pmm: Pmm,
    allocated_pages: usize,
    pages_ptr: *mut *const u8,
}
impl Heap {
    pub fn new(mut pmm: Pmm) -> Heap {
        let Some(pages_page_ptr) = pmm.alloc() else {
            panic!("PMM run out of free pages");
        };
        let mut heap = Heap {
            pmm,
            allocated_pages: 0,
            pages_ptr: pages_page_ptr as *mut *const u8,
        };

        heap.new_page();
        heap
    }

    pub fn malloc(&mut self, size: usize, align: usize) -> *const u8 {
        assert!(
            size + HEADER_SIZE <= PAGE_SIZE,
            "Cannot allocate {size} bytes, block too big"
        );

        let mut block = self.first_page_with_fit(size, align);

        self.try_split_block(&mut block, size);

        block.header.is_occupied = true;
        unsafe {
            block.write();
            block.aligned_data_ptr()
        }
    }

    fn try_split_block(&mut self, block: &mut Block, requested_capacity: usize) {
        const MIN_BLOCK_SIZE: usize = 16;

        let block_a_capacity =
            (block.alignment_offset + requested_capacity + HEADER_SIZE - 1) & !(HEADER_SIZE - 1);
        let remaining = block.header.capacity.saturating_sub(block_a_capacity);
        if remaining < HEADER_SIZE + MIN_BLOCK_SIZE {
            return;
        }

        let block_b_capacity = remaining - HEADER_SIZE;

        block.header.capacity = block_a_capacity;

        let block_b_header_ptr = unsafe { block.header.ptr.add(block.size()) };
        let block_b = Block::new(Header::new(block_b_header_ptr, block_b_capacity, false), 0);

        unsafe {
            block.write();
            block_b.write();
        }
    }

    fn first_page_with_fit(&mut self, size: usize, align: usize) -> Block {
        assert!(HEADER_SIZE + size <= PAGE_SIZE);

        for i in 0..self.allocated_pages {
            let page_ptr = unsafe { *self.pages_ptr.add(i) };
            if let Some(page_fit_ptr) = Heap::first_fit(page_ptr, size, align) {
                return page_fit_ptr;
            }
        }

        let new_page_ptr = self.new_page();
        Heap::first_fit(new_page_ptr, size, align)
            .expect("If block passed the assertion, it has to fit on an empty page")
    }

    fn first_fit(page_ptr: *const u8, size: usize, align: usize) -> Option<Block> {
        let mut block_ptr = page_ptr;

        while unsafe { block_ptr.offset_from(page_ptr) as usize } < PAGE_SIZE {
            let block_header = unsafe { Header::from_ptr(block_ptr) };

            if block_header.is_occupied {
                unsafe {
                    block_ptr = block_ptr.add(block_header.size());
                }
                continue;
            }

            let content_ptr = block_header.content_ptr();
            let data_loc = (content_ptr as usize + align + 1) & !(align - 1);
            let alignment_offset = data_loc - content_ptr as usize;

            let block_size = block_header.size();
            let block = Block::new(block_header, alignment_offset);

            if block.aligned_capacity() >= size {
                return Some(block);
            }

            unsafe {
                block_ptr = block_ptr.add(block_size);
            }
        }

        None
    }

    fn new_page(&mut self) -> *const u8 {
        let page_ptr = Heap::request_page(&mut self.pmm);
        unsafe {
            let pages_page_ptr = self.pages_ptr.add(self.allocated_pages);
            *pages_page_ptr = page_ptr;
        }
        self.allocated_pages += 1;
        page_ptr
    }

    fn request_page(pmm: &mut Pmm) -> *const u8 {
        let Some(page_ptr) = pmm.alloc() else {
            panic!("PMM run out of free pages");
        };

        // Create initial free block on a page, that spans the whole page.
        let block_header = Header::new(page_ptr, PAGE_SIZE - HEADER_SIZE, false);
        unsafe { block_header.write() };

        page_ptr
    }

    pub fn free(&mut self, aligned_data_ptr: *mut u8) {
        let block = unsafe { Block::from_aligned_data_ptr(aligned_data_ptr) };

        let next_block_header = unsafe {
            let next_block_header_ptr = block.header.ptr.add(block.size());
            Header::from_ptr(next_block_header_ptr)
        };
        let free_block_capacity = if next_block_header.is_occupied {
            block.header.capacity
        } else {
            block.header.capacity + next_block_header.size()
        };

        let free_block_header = Header::new(block.header.ptr, free_block_capacity, false);
        unsafe { free_block_header.write() };
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{heap::Heap, pmm::tests::setup_test_pmm};

    #[test_case]
    fn test_malloc_and_write() {
        let pmm = setup_test_pmm();
        let mut heap = Heap::new(pmm);

        let size = 16;
        let align = 8;
        let data_ptr = heap.malloc(size, align) as *mut u8;

        assert_eq!(data_ptr as usize % align, 0);

        unsafe {
            for i in 0..size {
                *data_ptr.add(i) = i as u8;
            }
            for i in 0..size {
                assert_eq!(*data_ptr.add(i), i as u8);
            }
        }
    }

    #[test_case]
    fn test_multiple_allocations() {
        let pmm = setup_test_pmm();
        let mut heap = Heap::new(pmm);

        let ptr1 = heap.malloc(16, 8);
        let ptr2 = heap.malloc(8, 4);
        let ptr3 = heap.malloc(9, 3);

        assert_ne!(ptr1, ptr2);
        assert_ne!(ptr1, ptr3);
        assert_ne!(ptr2, ptr3);
    }

    #[test_case]
    fn test_large_alignment() {
        let pmm = setup_test_pmm();
        let mut heap = Heap::new(pmm);

        let align = 64;
        let ptr = heap.malloc(8, align);

        assert_eq!(ptr as usize % align, 0);
    }
}
