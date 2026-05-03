mod block;
mod header;

use crate::{
    heap::{
        block::{Block, try_split_block},
        header::{Header, HeadersIterator},
    },
    memory_manager::{MemoryManager, PAGE_SIZE},
};

fn alignment_offset_from(ptr: *const u8, align: usize) -> usize {
    let rem = ptr as usize % align;
    if rem == 0 { 0 } else { align - rem }
}

const PAGES_FOR_ALLOC: usize = 20; // Who needs more than 20 pages, lol
pub struct Heap<M: MemoryManager> {
    mm: M,
    allocated_pages: usize,
    pages: [*const u8; PAGES_FOR_ALLOC],
}
impl<M: MemoryManager> Heap<M> {
    pub fn new(mut mm: M) -> Self {
        let initial_page = Self::request_page(&mut mm);
        let initial_block_header = Header::from_ptr(initial_page as *const Header);
        let initial_block = Block::occupied(
            initial_page as *const u8,
            initial_block_header.payload() - Header::SIZE,
            Header::SIZE,
        );

        let (pages_block, free_block) =
            try_split_block(initial_block, size_of::<usize>() * PAGES_FOR_ALLOC);
        unsafe {
            pages_block.write();
            if let Some(free_block) = free_block {
                free_block.write();
            }
        };

        let mut pages = [0 as *const u8; PAGES_FOR_ALLOC];
        pages[0] = initial_page as *const u8;

        Self {
            mm,
            allocated_pages: 1,
            pages,
        }
    }

    pub fn malloc(&mut self, size: usize, align: usize) -> *const u8 {
        assert!(
            size + Header::SIZE <= PAGE_SIZE,
            "Cannot allocate {size} bytes, block too big"
        );

        let block = self.first_fit(size, align);
        let (block_a, block_b) = try_split_block(block, size);

        unsafe {
            block_a.write();
            if let Some(block_b) = block_b {
                block_b.write();
            }
        };
        block_a.content_ptr()
    }

    pub fn free(&mut self, aligned_data_ptr: *mut u8) {
        let block = Block::from_aligned_data_ptr(aligned_data_ptr);

        let next_block_header_ptr = unsafe { block.ptr().add(block.size()) };
        let next_block_header = Header::from_ptr(next_block_header_ptr as *const Header);
        let free_block_capacity = if next_block_header.is_occupied() {
            block.content_offset() + block.capacity()
        } else {
            block.content_offset() + block.capacity() + next_block_header.size()
        };

        let free_block = Block::free(block.ptr(), free_block_capacity);
        unsafe { free_block.write() };
    }

    fn get_pages(&self) -> impl Iterator<Item = *const u8> {
        (0..self.allocated_pages).map(|i| self.pages[i])
    }

    fn first_fit(&mut self, size: usize, align: usize) -> Block {
        assert!(Header::SIZE + size <= PAGE_SIZE);

        if let Some(fit) = self
            .get_pages()
            .find_map(|page_ptr| Self::first_page_fit(page_ptr, size, align))
        {
            return fit;
        }

        let new_page_ptr = self.new_page();
        Self::first_page_fit(new_page_ptr, size, align)
            .expect("If block passed the assertion, it has to fit on an empty page")
    }

    fn first_page_fit(page_ptr: *const u8, size: usize, align: usize) -> Option<Block> {
        HeadersIterator::new(page_ptr)
            .filter(|(header, _)| !header.is_occupied())
            .map(|(header, ptr)| {
                let alignment_offset =
                    alignment_offset_from(unsafe { ptr.add(Header::SIZE) }, align);
                let content_alignment_offset = if alignment_offset == 0 {
                    align
                } else {
                    alignment_offset
                };
                (header, ptr, content_alignment_offset)
            })
            .find(|(header, _, content_alignment_offset)| {
                header.payload() >= content_alignment_offset + size
            })
            .map(|(header, ptr, content_alignment_offset)| {
                Block::occupied(
                    ptr,
                    header.payload() - content_alignment_offset,
                    content_alignment_offset,
                )
            })
    }

    fn new_page(&mut self) -> *const u8 {
        assert!(
            self.allocated_pages < PAGES_FOR_ALLOC,
            "Implementation constraint surpassed, too many pages allocated for heap!"
        );

        let page_ptr = Heap::request_page(&mut self.mm) as *const u8;
        self.pages[self.allocated_pages] = page_ptr;
        self.allocated_pages += 1;
        page_ptr
    }

    fn request_page(mm: &mut M) -> *const () {
        let Some(page_ptr) = mm.alloc() else {
            panic!("MM run out of free pages");
        };

        // Create initial free block on a page, that spans the whole page.
        // let block_header = Header::new(page_ptr as *const u8, PAGE_SIZE - Header::SIZE, false);
        let block = Block::free(page_ptr as *const u8, PAGE_SIZE - Header::SIZE);
        unsafe { block.write() };

        page_ptr
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{heap::Heap, memory_manager::setup_test_pmm};

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
    fn test_large_alignment() {
        let pmm = setup_test_pmm();
        let mut heap = Heap::new(pmm);

        let align = 64;
        let ptr = heap.malloc(8, align);

        assert_eq!(ptr as usize % align, 0);
    }

    #[test_case]
    fn test_multiple_allocations() {
        let pmm = setup_test_pmm();
        let mut heap = Heap::new(pmm);

        let ptr1 = heap.malloc(16, 8);
        assert_eq!(ptr1 as usize % 8, 0);
        let ptr2 = heap.malloc(8, 4);
        assert_eq!(ptr2 as usize % 4, 0);
        let ptr3 = heap.malloc(9, 3);
        assert_eq!(ptr3 as usize % 3, 0);

        assert_ne!(ptr1, ptr2);
        assert_ne!(ptr1, ptr3);
        assert_ne!(ptr2, ptr3);
    }

    #[test_case]
    fn test_allocation_on_multiple_pages() {
        use crate::memory_manager::PAGE_SIZE;

        let pmm = setup_test_pmm();
        let mut heap = Heap::new(pmm);

        let ptr1 = heap.malloc(100, 8);
        let ptr2 = heap.malloc(4000, 8);

        assert_ne!(ptr1, ptr2);
        assert_ne!(ptr1 as usize / PAGE_SIZE, ptr2 as usize / PAGE_SIZE);
    }
}
