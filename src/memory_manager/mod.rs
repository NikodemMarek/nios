mod page_table;
mod page_table_entry;
mod pmm;
mod vmm;

pub use page_table::PageTable;
pub use pmm::{Pmm, init_pmm};
pub use vmm::Vmm;

pub const PAGE_SIZE: usize = 4096;

pub trait MemoryManager {
    fn alloc(&mut self) -> Option<*const ()>;
    fn free(&mut self, page_ptr: *const ());
}

#[cfg(test)]
pub use pmm::tests::setup_test_pmm;
