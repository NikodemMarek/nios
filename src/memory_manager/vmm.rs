use crate::memory_manager::{MemoryManager, Pmm, page_table::PageTable};

#[derive(Copy, Clone)]
pub struct Vmm {
    pmm: Pmm,
    root_page_table: PageTable,
}

impl Vmm {
    pub fn new(pmm: Pmm, root_page_table: PageTable) -> Self {
        Self {
            pmm,
            root_page_table,
        }
    }
}

impl MemoryManager for Vmm {
    fn alloc(&mut self) -> Option<*const ()> {
        let (l2, l1, l0) = self.root_page_table.add_page(&mut self.pmm)?;
        let virtual_address = (l2 << 30) | (l1 << 21) | (l0 << 12);

        Some(virtual_address as *const ())
    }

    fn free(&mut self, page_ptr: *const ()) {
        todo!()
    }
}
