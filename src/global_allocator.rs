use core::{
    alloc::GlobalAlloc,
    cell::{RefCell, RefMut},
};

use crate::heap::Heap;

pub struct GlobalAllocator(RefCell<Option<Heap>>);
impl GlobalAllocator {
    #[inline]
    pub const fn empty() -> Self {
        GlobalAllocator(RefCell::new(None))
    }
    #[inline]
    pub fn init(&self, heap: Heap) {
        *self.0.borrow_mut() = Some(heap);
    }

    #[inline]
    fn get(&self) -> core::cell::RefMut<'_, Heap> {
        RefMut::map(self.0.borrow_mut(), |mi| {
            mi.as_mut().expect("Allocator not initialized")
        })
    }
}
unsafe impl GlobalAlloc for GlobalAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.get().malloc(layout.size(), layout.align()) as *mut u8
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        self.get().free(ptr);
    }
}
unsafe impl Send for GlobalAllocator {}
unsafe impl Sync for GlobalAllocator {}
