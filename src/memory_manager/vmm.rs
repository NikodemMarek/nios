use crate::memory_manager::{MemoryManager, Pmm, page_table::PageTable};

#[derive(Copy, Clone)]
pub struct Vmm {
    pmm: Pmm,
    root_page_table: PageTable,
}

impl Vmm {
    pub fn init(pmm: Pmm, root_page_ptr: *const ()) -> Self {
        let root_page_table = PageTable::new_root(root_page_ptr);
        Self {
            pmm,
            root_page_table,
        }
    }
}

impl MemoryManager for Vmm {
    fn alloc(&mut self) -> Option<*const u8> {
        let (l2, l1, l0) = self.root_page_table.add_page(&mut self.pmm)?;
        let virtual_address = (l2 << 30) | (l1 << 21) | (l0 << 12);

        crate::println!("virtual_address: {virtual_address}");

        Some(virtual_address as *const u8)
    }

    fn free(&mut self, page_ptr: *const u8) {
        todo!()
    }
}
