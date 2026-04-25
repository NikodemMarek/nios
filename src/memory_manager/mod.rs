mod page_table;
mod page_table_entry;
mod pmm;
mod vmm;

use crate::{VIRT_BASE, memory_manager::page_table::PageTable};

pub use page_table::remove_kernel_identity_map;
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
    fn alloc(&mut self) -> Option<*const ()>;
    fn free(&mut self, page_ptr: *const ());
}

pub fn write_setup_page(pmm: &mut Pmm, root_page_table_ptr: *const ()) -> usize {
    let setup_page_ptr = pmm.alloc().expect("PMM out of pages") as *mut u64;
    let (bitmap_ptr, total_pages) = pmm.to_raw();
    unsafe {
        *setup_page_ptr = bitmap_ptr as u64;
        *setup_page_ptr.add(1) = total_pages as u64;
        *setup_page_ptr.add(2) = root_page_table_ptr as u64;
    }
    setup_page_ptr as usize
}
pub fn read_setup_page(setup_page_loc: usize) -> (Pmm, PageTable) {
    let memory_start_ptr = VIRT_BASE as *const u8;

    let setup_page_ptr = unsafe { memory_start_ptr.add(setup_page_loc) as *mut u64 };
    let bitmap_ptr = unsafe {
        let bitmap_loc = *setup_page_ptr as usize;
        memory_start_ptr.add(bitmap_loc)
    };
    let total_pages = unsafe { *setup_page_ptr.add(1) as usize };
    let root_page_table_ptr = unsafe {
        let root_page_table_loc = *setup_page_ptr.add(2) as usize;
        memory_start_ptr.add(root_page_table_loc)
    };

    let mut pmm = Pmm::from_raw(
        memory_start_ptr as *const (),
        bitmap_ptr as *const (),
        total_pages,
    );
    let root_page_table = PageTable::new_root(root_page_table_ptr as *const (), true);

    // cleanup the setup page
    unsafe {
        *setup_page_ptr = 0;
        *setup_page_ptr.add(1) = 0;
        *setup_page_ptr.add(2) = 0;
    }
    pmm.free(setup_page_ptr as *const ());

    (pmm, root_page_table)
}

#[cfg(test)]
pub use pmm::tests::setup_test_pmm;
