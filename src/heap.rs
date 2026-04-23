use crate::memory_manager::{MemoryManager, PAGE_SIZE};

fn alignment_offset_from(ptr: *const u8, align: usize) -> usize {
    let rem = ptr as usize % align;
    if rem == 0 { 0 } else { align - rem }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Header(u64);
impl Header {
    const SIZE: usize = size_of::<usize>();

    fn new(capacity: usize, is_occupied: bool) -> Self {
        Self(if is_occupied {
            capacity as u64 | (1 << 63)
        } else {
            capacity as u64 & !(1 << 63)
        })
    }

    pub fn from_ptr(ptr: *const Header) -> Self {
        if ptr.is_null() {
            Header(0b0)
        } else {
            unsafe { *ptr }
        }
    }

    // Header capacity is not the same as Block capacity, it also includes alignment offset for the
    // block content.
    #[inline]
    fn capacity(&self) -> usize {
        (self.0 & !(1 << 63)) as usize
    }

    #[inline]
    fn is_occupied(&self) -> bool {
        (self.0 >> 63) & 1 == 1
    }

    #[inline]
    fn size(&self) -> usize {
        Header::SIZE + self.capacity()
    }
}

enum Block {
    Free {
        ptr: *const u8,
        capacity: usize,
    },
    Occupied {
        ptr: *const u8,
        capacity: usize,
        alignment_offset: usize,
    },
}
impl Block {
    fn free(ptr: *const u8, capacity: usize) -> Self {
        Self::Free { ptr, capacity }
    }
    fn occupied(ptr: *const u8, capacity: usize, alignment_offset: usize) -> Self {
        Self::Occupied {
            ptr,
            capacity,
            alignment_offset,
        }
    }

    fn is_occupied(&self) -> bool {
        match self {
            Self::Free { .. } => false,
            Self::Occupied { .. } => true,
        }
    }
    fn ptr(&self) -> *const u8 {
        match self {
            Self::Free { ptr, .. } => *ptr,
            Self::Occupied { ptr, .. } => *ptr,
        }
    }
    fn capacity(&self) -> usize {
        match self {
            Self::Free { capacity, .. } => *capacity,
            Self::Occupied { capacity, .. } => *capacity,
        }
    }
    fn alignment_offset(&self) -> usize {
        match self {
            Self::Free { .. } => 0,
            Self::Occupied {
                alignment_offset, ..
            } => *alignment_offset,
        }
    }

    fn size(&self) -> usize {
        match self {
            Self::Free { capacity, .. } => Header::SIZE + capacity,
            Self::Occupied {
                capacity,
                alignment_offset,
                ..
            } => Header::SIZE + alignment_offset + capacity,
        }
    }

    fn content_ptr(&self) -> *const u8 {
        match self {
            Self::Free { ptr, .. } => unsafe { ptr.add(Header::SIZE) },
            Self::Occupied {
                ptr,
                alignment_offset,
                ..
            } => unsafe { ptr.add(Header::SIZE + *alignment_offset) },
        }
    }

    unsafe fn write(&self) {
        let is_occupied = self.is_occupied();
        let ptr = self.ptr();

        unsafe {
            *(ptr as *mut Header) =
                Header::new(self.capacity() + self.alignment_offset(), is_occupied);

            if let Self::Occupied {
                alignment_offset, ..
            } = self
            {
                *(ptr.add(Header::SIZE + alignment_offset - 1) as *mut u8) =
                    *alignment_offset as u8;
            }
        }
    }

    fn from_aligned_data_ptr(aligned_data_ptr: *mut u8) -> Self {
        let alignment_offset = unsafe {
            let offset_ptr = aligned_data_ptr.sub(1);
            *offset_ptr as usize
        };

        let block_header_ptr = unsafe { aligned_data_ptr.sub(Header::SIZE + alignment_offset) };
        let header = Header::from_ptr(block_header_ptr as *const Header);

        Self::Occupied {
            ptr: block_header_ptr,
            capacity: header.capacity() - alignment_offset,
            alignment_offset,
        }
    }
}

fn try_split_block(block_a: Block, requested_capacity: usize) -> (Block, Option<Block>) {
    const MIN_BLOCK_SIZE: usize = 16;

    let capacity_to_split = block_a.capacity();
    if capacity_to_split - requested_capacity < Header::SIZE + MIN_BLOCK_SIZE {
        return (block_a, None);
    }

    let unaligned_block_b_ptr = unsafe { block_a.content_ptr().add(requested_capacity) };
    let block_b_header_alignment_offset =
        alignment_offset_from(unaligned_block_b_ptr, Header::SIZE);
    let block_b_ptr = unsafe { unaligned_block_b_ptr.add(block_b_header_alignment_offset) };

    let block_a = Block::occupied(
        block_a.ptr(),
        requested_capacity + block_b_header_alignment_offset,
        block_a.alignment_offset(),
    );

    let block_b_capacity = capacity_to_split - block_a.capacity() - Header::SIZE;
    let block_b = Block::free(block_b_ptr, block_b_capacity);

    (block_a, Some(block_b))
}

struct HeadersIterator {
    current_ptr: Option<*const u8>,
    start_ptr: usize,
}
impl HeadersIterator {
    fn new(ptr: *const u8) -> Self {
        Self {
            current_ptr: Some(ptr),
            start_ptr: ptr as usize,
        }
    }
}
impl Iterator for HeadersIterator {
    type Item = (Header, *const u8);

    fn next(&mut self) -> Option<Self::Item> {
        let current_ptr = self.current_ptr?;
        let header = Header::from_ptr(current_ptr as *const Header);

        let next_ptr = unsafe { current_ptr.add(header.size()) };
        let next_ptr_offset = next_ptr as usize - self.start_ptr;
        if next_ptr_offset > PAGE_SIZE {
            self.current_ptr = None;
        } else {
            self.current_ptr = Some(next_ptr);
        }

        Some((header, current_ptr))
    }
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
            initial_block_header.capacity(),
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
            block.capacity()
        } else {
            block.capacity() + next_block_header.size()
        };

        let free_block = Block::free(next_block_header_ptr, free_block_capacity);
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
                let alignment_offset = if alignment_offset == 0 {
                    align
                } else {
                    alignment_offset
                };
                (header, ptr, alignment_offset)
            })
            .find(|(header, _, alignment_offset)| header.capacity() >= alignment_offset + size)
            .map(|(header, ptr, alignment_offset)| {
                Block::occupied(ptr, header.capacity(), alignment_offset)
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

    fn request_page(pmm: &mut M) -> *const () {
        let Some(page_ptr) = pmm.alloc() else {
            panic!("PMM run out of free pages");
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
    use crate::{
        heap::{Block, Heap, try_split_block},
        memory_manager::setup_test_pmm,
    };

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

    #[test_case]
    fn test_block_split() {
        let block = Block::occupied(0 as *const u8, 100, 0);
        let (block_a, block_b) = try_split_block(block, 90);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 8);
        assert_eq!(block_a.capacity(), 100);
        assert_eq!(block_a.alignment_offset(), 0);
        assert!(block_b.is_none());

        let block = Block::occupied(0 as *const u8, 100, 0);
        let (block_a, block_b) = try_split_block(block, 50);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 8);
        assert_eq!(block_a.capacity(), 56);
        assert_eq!(block_a.alignment_offset(), 0);
        if let Some(block_b) = block_b {
            assert_eq!(block_b.ptr() as usize, 64);
            assert_eq!(block_b.capacity(), 36);
            assert_eq!(block_b.alignment_offset(), 0);
        }

        let block = Block::occupied(0 as *const u8, 100, 6);
        let (block_a, block_b) = try_split_block(block, 50);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 14);
        assert_eq!(block_a.capacity(), 50);
        assert_eq!(block_a.alignment_offset(), 6);
        if let Some(block_b) = block_b {
            assert_eq!(block_b.ptr() as usize, 64);
            assert_eq!(block_b.capacity(), 42);
            assert_eq!(block_b.alignment_offset(), 0);
        }

        let block = Block::occupied(0 as *const u8, 100, 3);
        let (block_a, block_b) = try_split_block(block, 50);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 11);
        assert_eq!(block_a.capacity(), 53);
        assert_eq!(block_a.alignment_offset(), 3);
        if let Some(block_b) = block_b {
            assert_eq!(block_b.ptr() as usize, 64);
            assert_eq!(block_b.capacity(), 39);
            assert_eq!(block_b.alignment_offset(), 0);
        }
    }
}
