use crate::memory_manager::PAGE_SIZE;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Header(u64);
impl Header {
    pub const SIZE: usize = size_of::<usize>();

    pub fn new(payload: usize, is_occupied: bool) -> Self {
        Self(if is_occupied {
            payload as u64 | (1 << 63)
        } else {
            payload as u64 & !(1 << 63)
        })
    }

    pub fn from_ptr(ptr: *const Header) -> Self {
        if ptr.is_null() {
            Header(0b0)
        } else {
            unsafe { *ptr }
        }
    }

    #[inline]
    pub fn payload(&self) -> usize {
        (self.0 & !(1 << 63)) as usize
    }

    #[inline]
    pub fn is_occupied(&self) -> bool {
        (self.0 >> 63) & 1 == 1
    }

    #[inline]
    pub fn size(&self) -> usize {
        Header::SIZE + self.payload()
    }
}

pub struct HeadersIterator {
    current_ptr: Option<*const u8>,
    start_ptr: usize,
}
impl HeadersIterator {
    pub fn new(ptr: *const u8) -> Self {
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
        if next_ptr_offset >= PAGE_SIZE {
            self.current_ptr = None;
        } else {
            self.current_ptr = Some(next_ptr);
        }

        Some((header, current_ptr))
    }
}
