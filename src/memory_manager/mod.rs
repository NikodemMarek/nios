mod pmm;

pub use pmm::Pmm;

pub const PAGE_SIZE: usize = 4096;

pub trait MemoryManager {
    fn alloc(&mut self) -> Option<*const u8>;
    fn free(&mut self, page_ptr: *const u8);
}

#[cfg(test)]
pub use pmm::tests::setup_test_pmm;
