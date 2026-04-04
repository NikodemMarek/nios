mod pmm;
mod pte;
mod vmm;

pub use pmm::Pmm;
pub use pte::{Pte, PteAttributes};

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

pub fn satp(root_page_ptr: *const ()) -> u64 {
    let ppn = (root_page_ptr as u64) >> 12;
    (0b1000u64 << 60) | ppn
}

#[cfg(test)]
pub use pmm::tests::setup_test_pmm;
