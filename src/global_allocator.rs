use core::{
    alloc::GlobalAlloc,
    cell::{RefCell, RefMut},
};

use crate::{heap::Heap, memory_manager::MemoryManager};

pub struct GlobalAllocator<M: MemoryManager>(RefCell<Option<Heap<M>>>);
impl<M: MemoryManager> GlobalAllocator<M> {
    #[inline]
    pub const fn empty() -> Self {
        GlobalAllocator(RefCell::new(None))
    }
    #[inline]
    pub fn init(&self, heap: Heap<M>) {
        *self.0.borrow_mut() = Some(heap);
    }

    #[inline]
    fn get(&self) -> core::cell::RefMut<'_, Heap<M>> {
        RefMut::map(self.0.borrow_mut(), |mi| {
            mi.as_mut().expect("Allocator not initialized")
        })
    }
}
unsafe impl<M: MemoryManager> GlobalAlloc for GlobalAllocator<M> {
    #[inline]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.get().malloc(layout.size(), layout.align()) as *mut u8
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        self.get().free(ptr);
    }
}
unsafe impl<M: MemoryManager> Send for GlobalAllocator<M> {}
unsafe impl<M: MemoryManager> Sync for GlobalAllocator<M> {}
