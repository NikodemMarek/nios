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
        let virtual_address = ((virtual_address as i64) << 25 >> 25) as u64;

        unsafe {
            core::arch::asm!("sfence.vma zero, zero");
        }

        Some(virtual_address as *const ())
    }

    fn free(&mut self, page_ptr: *const ()) {
        todo!()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::memory_manager::{MemoryManager, PageTable, setup_test_pmm, vmm::Vmm};

    pub fn setup_test_vmm() -> Vmm {
        let mut pmm = setup_test_pmm();
        let root_page_table_ptr = pmm.alloc().unwrap();
        let root_page_table = PageTable::new_root(root_page_table_ptr);

        Vmm::new(pmm, root_page_table)
    }
}
