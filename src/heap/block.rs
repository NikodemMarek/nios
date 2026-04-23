use crate::heap::{alignment_offset_from, header::Header};

pub enum Block {
    Free {
        ptr: *const u8,
        capacity: usize,
    },
    Occupied {
        ptr: *const u8,
        capacity: usize,
        content_offset: usize,
    },
}
impl Block {
    pub fn free(ptr: *const u8, capacity: usize) -> Self {
        Self::Free { ptr, capacity }
    }
    pub fn occupied(ptr: *const u8, capacity: usize, content_offset: usize) -> Self {
        Self::Occupied {
            ptr,
            capacity,
            content_offset,
        }
    }

    fn is_occupied(&self) -> bool {
        match self {
            Self::Free { .. } => false,
            Self::Occupied { .. } => true,
        }
    }
    pub fn ptr(&self) -> *const u8 {
        match self {
            Self::Free { ptr, .. } => *ptr,
            Self::Occupied { ptr, .. } => *ptr,
        }
    }
    pub fn capacity(&self) -> usize {
        match self {
            Self::Free { capacity, .. } => *capacity,
            Self::Occupied { capacity, .. } => *capacity,
        }
    }
    pub fn content_offset(&self) -> usize {
        match self {
            Self::Free { .. } => 0,
            Self::Occupied { content_offset, .. } => *content_offset,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Free { capacity, .. } => Header::SIZE + capacity,
            Self::Occupied {
                capacity,
                content_offset,
                ..
            } => Header::SIZE + content_offset + capacity,
        }
    }

    pub fn content_ptr(&self) -> *const u8 {
        match self {
            Self::Free { ptr, .. } => unsafe { ptr.add(Header::SIZE) },
            Self::Occupied {
                ptr,
                content_offset,
                ..
            } => unsafe { ptr.add(Header::SIZE + *content_offset) },
        }
    }

    pub unsafe fn write(&self) {
        let is_occupied = self.is_occupied();
        let ptr = self.ptr();

        unsafe {
            *(ptr as *mut Header) =
                Header::new(self.capacity() + self.content_offset(), is_occupied);

            if let Self::Occupied { content_offset, .. } = self {
                *(ptr.add(Header::SIZE + content_offset - 1) as *mut u8) = *content_offset as u8;
            }
        }
    }

    pub fn from_aligned_data_ptr(aligned_data_ptr: *mut u8) -> Self {
        let content_offset = unsafe {
            let offset_ptr = aligned_data_ptr.sub(1);
            *offset_ptr as usize
        };

        let block_header_ptr = unsafe { aligned_data_ptr.sub(Header::SIZE + content_offset) };
        let header = Header::from_ptr(block_header_ptr as *const Header);

        Self::Occupied {
            ptr: block_header_ptr,
            capacity: header.payload() - content_offset,
            content_offset,
        }
    }
}

pub fn try_split_block(block_a: Block, requested_capacity: usize) -> (Block, Option<Block>) {
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
        block_a.content_offset(),
    );

    let block_b_capacity = capacity_to_split - block_a.capacity() - Header::SIZE;
    let block_b = Block::free(block_b_ptr, block_b_capacity);

    (block_a, Some(block_b))
}

#[cfg(test)]
pub mod tests {
    use super::{Block, try_split_block};

    #[test_case]
    fn test_block_split() {
        let block = Block::occupied(0 as *const u8, 100, 0);
        let (block_a, block_b) = try_split_block(block, 90);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 8);
        assert_eq!(block_a.capacity(), 100);
        assert_eq!(block_a.content_offset(), 0);
        assert!(block_b.is_none());

        let block = Block::occupied(0 as *const u8, 100, 0);
        let (block_a, block_b) = try_split_block(block, 50);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 8);
        assert_eq!(block_a.capacity(), 56);
        assert_eq!(block_a.content_offset(), 0);
        if let Some(block_b) = block_b {
            assert_eq!(block_b.ptr() as usize, 64);
            assert_eq!(block_b.capacity(), 36);
            assert_eq!(block_b.content_offset(), 0);
        }

        let block = Block::occupied(0 as *const u8, 100, 6);
        let (block_a, block_b) = try_split_block(block, 50);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 14);
        assert_eq!(block_a.capacity(), 50);
        assert_eq!(block_a.content_offset(), 6);
        if let Some(block_b) = block_b {
            assert_eq!(block_b.ptr() as usize, 64);
            assert_eq!(block_b.capacity(), 42);
            assert_eq!(block_b.content_offset(), 0);
        }

        let block = Block::occupied(0 as *const u8, 100, 3);
        let (block_a, block_b) = try_split_block(block, 50);
        assert_eq!(block_a.ptr() as usize, 0);
        assert_eq!(block_a.content_ptr() as usize, 11);
        assert_eq!(block_a.capacity(), 53);
        assert_eq!(block_a.content_offset(), 3);
        if let Some(block_b) = block_b {
            assert_eq!(block_b.ptr() as usize, 64);
            assert_eq!(block_b.capacity(), 39);
            assert_eq!(block_b.content_offset(), 0);
        }
    }
}
