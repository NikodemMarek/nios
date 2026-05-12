use crate::memory_manager::{Pmm, VirtualAddress, page_table::PageTable};

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

    pub fn alloc(&mut self) -> Option<VirtualAddress> {
        let virtual_address = self.root_page_table.add_page(&mut self.pmm)?;

        unsafe {
            core::arch::asm!("sfence.vma zero, zero");
        }

        Some(virtual_address)
    }

    pub fn free(&mut self, page_addr: VirtualAddress) {
        todo!()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::memory_manager::{PageTable, setup_test_pmm, vmm::Vmm};

    pub fn setup_test_vmm() -> Vmm {
        let mut pmm = setup_test_pmm();
        let root_page_table = PageTable::new_root(pmm.alloc().unwrap().into());

        Vmm::new(pmm, root_page_table)
    }
}
