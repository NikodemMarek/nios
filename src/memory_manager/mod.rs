mod page_table;
mod page_table_entry;
mod pmm;
mod vmm;

pub use page_table::init as init_page_table;
pub use page_table_entry::PageTableEntry;
pub use pmm::Pmm;
pub use vmm::Vmm;

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
    static _memory_start: u8;
    static _memory_end: u8;
}

pub const PAGE_SIZE: usize = 4096;

pub trait MemoryManager {
    fn alloc(&mut self) -> Option<*const u8>;
    fn free(&mut self, page_ptr: *const u8);
}

#[cfg(test)]
pub use pmm::tests::setup_test_pmm;
