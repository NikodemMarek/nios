use crate::memory_manager::{MemoryManager, PageTableEntry, PageTableEntryAttributes, Pmm};

#[derive(Copy, Clone)]
pub struct PageTableLevelRoot;
#[derive(Copy, Clone)]
struct PageTableLevelL1;
#[derive(Copy, Clone)]
struct PageTableLevelL0;

#[derive(Copy, Clone)]
pub struct PageTable<PageTableLevel = PageTableLevelRoot> {
    level: core::marker::PhantomData<PageTableLevel>,
    ptr: *const (),
}
impl<L> PageTable<L> {
    fn new(ptr: *const ()) -> Self {
        Self {
            ptr,
            level: core::marker::PhantomData,
        }
    }

    fn get_pte(&self, i: usize) -> PageTableEntry {
        assert!(i < 512, "page table entry index {i} too high");
        PageTableEntry::from_ptr(unsafe { (self.ptr as *const u64).add(i) })
    }
    fn get_ptes(&self) -> impl Iterator<Item = PageTableEntry> {
        (0..512).map(|i| self.get_pte(i))
    }

    fn add_page_table(&mut self, pmm: &mut Pmm) -> Option<usize> {
        let page_table_ptr = pmm.alloc().expect("PMM out of pages");

        let page_table_pte = PageTableEntry::new(
            page_table_ptr as *const (),
            PageTableEntryAttributes::default(),
        );
        let free_index = self
            .get_ptes()
            .enumerate()
            .find_map(|(i, pte)| (!pte.is_valid()).then_some(i))?;

        unsafe {
            let page_table_pte_ptr = (self.ptr as *mut u64).add(free_index);
            *page_table_pte_ptr = page_table_pte.0;
        }

        Some(free_index)
    }
}
impl PageTable<PageTableLevelRoot> {
    pub fn new_root(ptr: *const ()) -> PageTable<PageTableLevelRoot> {
        PageTable {
            ptr,
            level: core::marker::PhantomData::<PageTableLevelRoot>,
        }
    }

    pub fn add_page(&mut self, pmm: &mut Pmm) -> Option<(usize, usize, usize)> {
        let l1_add_page_result = self
            .get_ptes()
            .enumerate()
            .filter(|(_, pte)| pte.is_valid() && !pte.is_leaf())
            .find_map(|(i, pte)| {
                let l1_page_table_ptr = pte.page_ptr();
                let mut l1_page_table = PageTable::<PageTableLevelL1>::new(l1_page_table_ptr);

                l1_page_table.add_page(pmm).map(|p| (i, p.0, p.1))
            });

        if l1_add_page_result.is_some() {
            return l1_add_page_result;
        }

        // allocate new page for page table on this level
        let new_l1_page_index = self.add_page_table(pmm)?;
        let new_l1_page_pte = self.get_pte(new_l1_page_index);

        let mut new_l1_page = PageTable::<PageTableLevelL1>::new(new_l1_page_pte.page_ptr());
        let l1_add_page_result = new_l1_page
            .add_page(pmm)
            .expect("new page table cannot be full");

        Some((
            new_l1_page_index,
            l1_add_page_result.0,
            l1_add_page_result.1,
        ))
    }
}
impl PageTable<PageTableLevelL1> {
    fn add_page(&mut self, pmm: &mut super::Pmm) -> Option<(usize, usize)> {
        let l0_add_page_result = self
            .get_ptes()
            .enumerate()
            .filter(|(_, pte)| pte.is_valid() && !pte.is_leaf())
            .find_map(|(i, pte)| {
                let l0_page_table_ptr = pte.page_ptr();
                let mut l0_page_table = PageTable::<PageTableLevelL0>::new(l0_page_table_ptr);

                l0_page_table.add_page(pmm).map(|p| (i, p))
            });

        if l0_add_page_result.is_some() {
            return l0_add_page_result;
        }

        // allocate new page for page table on this level
        let new_l0_page_index = self.add_page_table(pmm)?;
        let new_l0_page_pte = self.get_pte(new_l0_page_index);

        let mut new_l0_page = PageTable::<PageTableLevelL0>::new(new_l0_page_pte.page_ptr());
        let l0_page_index = new_l0_page
            .add_page(pmm)
            .expect("new page table cannot be full");

        Some((new_l0_page_index, l0_page_index))
    }
}
impl PageTable<PageTableLevelL0> {
    fn add_page(&mut self, pmm: &mut super::Pmm) -> Option<usize> {
        let free_index = self
            .get_ptes()
            .enumerate()
            .find_map(|(i, pte)| (!pte.is_valid()).then_some(i))?;

        let new_page_ptr = pmm.alloc().expect("PMM out of pages");
        let new_page_pte = PageTableEntry::new(
            new_page_ptr as *const (),
            PageTableEntryAttributes::default()
                .dirty()
                .accessed()
                .execute()
                .write()
                .read(),
        );

        unsafe {
            let new_page_pte_ptr = (self.ptr as *mut u64).add(free_index);
            *new_page_pte_ptr = new_page_pte.0;
        }

        Some(free_index)
    }
}
impl<L> core::fmt::Display for PageTable<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.get_ptes().try_for_each(|pte| writeln!(f, "{}", pte))
    }
}
