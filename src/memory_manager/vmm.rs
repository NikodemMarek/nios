use crate::memory_manager::{MemoryManager, PageTableEntry, PageTableEntryAttributes, Pmm};

#[derive(Copy, Clone)]
pub struct Vmm {
    pmm: Pmm,
    root_page_ptr: *const (),
    allocated: usize,
}

impl Vmm {
    pub fn init(mut pmm: Pmm, root_page_ptr: *const ()) -> Self {
        let l1_page_ptr = pmm.alloc().expect("PMM out of pages");
        let l0_page_ptr = pmm.alloc().expect("PMM out of pages");
        let pte_attrs = PageTableEntryAttributes::default();
        let l1_page_pte = PageTableEntry::new(l1_page_ptr as *const (), pte_attrs);
        let l0_page_pte = PageTableEntry::new(l0_page_ptr as *const (), pte_attrs);

        unsafe {
            let root_page_pte_ptr = (root_page_ptr as *mut u64).add(1);
            *root_page_pte_ptr = l1_page_pte.0;

            let l1_page_pte_ptr = l1_page_ptr as *mut u64;
            *l1_page_pte_ptr = l0_page_pte.0;
        };

        Self {
            pmm,
            root_page_ptr,
            allocated: 0,
        }
    }
}

impl MemoryManager for Vmm {
    fn alloc(&mut self) -> Option<*const u8> {
        let new_page_ptr = self.pmm.alloc()?;

        let pte_attrs = PageTableEntryAttributes::default()
            .dirty()
            .accessed()
            .execute()
            .write()
            .read();
        let new_page_pte = PageTableEntry::new(new_page_ptr as *const (), pte_attrs);

        unsafe {
            let root_page_pte_ptr = (self.root_page_ptr as *const u64).add(1);
            let root_page_pte = PageTableEntry(*root_page_pte_ptr);

            let l1_page_pte_ptr = root_page_pte.page_ptr() as *const u64;
            let l1_page_pte = PageTableEntry(*l1_page_pte_ptr);

            let l0_page_pte_ptr = (l1_page_pte.page_ptr() as *mut u64).add(self.allocated);
            *l0_page_pte_ptr = new_page_pte.0;
        }

        let virtual_address = (0b000000001 << 30) | (self.allocated << 12);
        self.allocated += 1;

        Some(virtual_address as *const u8)
    }

    fn free(&mut self, page_ptr: *const u8) {
        todo!()
    }
}
