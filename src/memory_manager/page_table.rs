use crate::{
    KERNEL_OFFSET, PHYS_BASE, VIRT_BASE,
    memory_manager::{MemoryManager, Pmm, page_table_entry::PageTableEntry},
};

#[derive(Copy, Clone)]
pub struct PageTableLevelRoot;
#[derive(Copy, Clone)]
struct PageTableLevelL1;
#[derive(Copy, Clone)]
struct PageTableLevelL0;

#[derive(Copy, Clone)]
pub struct PageTable<PageTableLevel = PageTableLevelRoot> {
    level: core::marker::PhantomData<PageTableLevel>,
    ptr: *const PageTableEntry,
    offset: usize,
}
impl<L> PageTable<L> {
    fn new(ptr: *const (), offset: usize) -> Self {
        Self {
            level: core::marker::PhantomData,
            ptr: ptr as *const PageTableEntry,
            offset,
        }
    }

    fn page_table_ptr(&self, ptr: *const ()) -> *const () {
        (self.offset + ptr as usize) as *const ()
    }

    fn get_pte(&self, i: usize) -> PageTableEntry {
        assert!(i < 512, "page table entry index {i} too high");
        PageTableEntry::from_ptr(unsafe { self.ptr.add(i) })
    }
    fn set_pte(&mut self, index: usize, pte: PageTableEntry) {
        unsafe {
            let page_pte_ptr = self.ptr.add(index) as *mut PageTableEntry;
            *page_pte_ptr = pte;
        }
    }

    fn get_ptes(&self) -> impl Iterator<Item = PageTableEntry> {
        (0..512).map(|i| self.get_pte(i))
    }
    fn free_index(&self) -> Option<usize> {
        self.get_ptes()
            .enumerate()
            .find_map(|(i, pte)| (!pte.is_valid()).then_some(i))
    }

    fn add_page_table(&mut self, pmm: &mut Pmm) -> Option<usize> {
        let free_index = self.free_index()?;

        let page_table_ptr = pmm.alloc().expect("PMM out of pages");
        let page_table_pte = PageTableEntry::page_table(page_table_ptr);

        self.set_pte(free_index, page_table_pte);

        Some(free_index)
    }
    fn add_leaf_page(&mut self, pmm: &mut super::Pmm) -> Option<usize> {
        let free_index = self.free_index()?;

        let leaf_page_ptr = pmm.alloc().expect("PMM out of pages");
        let leaf_page_pte = PageTableEntry::leaf(leaf_page_ptr);

        self.set_pte(free_index, leaf_page_pte);

        Some(free_index)
    }
}
impl PageTable<PageTableLevelRoot> {
    pub fn new_root(ptr: *const (), is_virtual: bool) -> PageTable<PageTableLevelRoot> {
        PageTable {
            level: core::marker::PhantomData::<PageTableLevelRoot>,
            ptr: ptr as *const PageTableEntry,
            offset: if is_virtual { VIRT_BASE } else { PHYS_BASE },
        }
    }

    pub fn satp(&self) -> u64 {
        let ppn = (self.ptr as u64) >> 12;
        (0b1000u64 << 60) | ppn
    }

    pub fn add_page(&mut self, pmm: &mut Pmm) -> Option<(usize, usize, usize)> {
        let l1_add_page_result = self
            .get_ptes()
            .enumerate()
            .filter(|(_, pte)| pte.is_valid() && !pte.is_leaf())
            .find_map(|(i, pte)| {
                // PTEs store physical addresses; convert to virtual to access the page table.
                let l1_page_table_ptr = self.page_table_ptr(pte.page_ptr());
                let mut l1_page_table =
                    PageTable::<PageTableLevelL1>::new(l1_page_table_ptr, self.offset);

                l1_page_table.add_page(pmm).map(|p| (i, p.0, p.1))
            });

        if l1_add_page_result.is_some() {
            return l1_add_page_result;
        }

        // allocate new page for page table on this level
        let new_l1_page_index = self.add_page_table(pmm)?;
        let new_l1_page_pte = self.get_pte(new_l1_page_index);

        let mut new_l1_page = PageTable::<PageTableLevelL1>::new(
            self.page_table_ptr(new_l1_page_pte.page_ptr()),
            self.offset,
        );
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
                // PTEs store physical addresses; convert to virtual to access the page table.
                let l0_page_table_ptr = self.page_table_ptr(pte.page_ptr());
                let mut l0_page_table =
                    PageTable::<PageTableLevelL0>::new(l0_page_table_ptr, self.offset);

                l0_page_table.add_page(pmm).map(|p| (i, p))
            });

        if l0_add_page_result.is_some() {
            return l0_add_page_result;
        }

        // allocate new page for page table on this level
        let new_l0_page_index = self.add_page_table(pmm)?;
        let new_l0_page_pte = self.get_pte(new_l0_page_index);

        let mut new_l0_page = PageTable::<PageTableLevelL0>::new(
            self.page_table_ptr(new_l0_page_pte.page_ptr()),
            self.offset,
        );
        let l0_page_index = new_l0_page
            .add_page(pmm)
            .expect("new page table cannot be full");

        Some((new_l0_page_index, l0_page_index))
    }
}
impl PageTable<PageTableLevelL0> {
    fn add_page(&mut self, pmm: &mut super::Pmm) -> Option<usize> {
        self.add_leaf_page(pmm)
    }
}
impl<L> core::fmt::Display for PageTable<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.get_ptes()
            .enumerate()
            .try_for_each(|(i, pte)| writeln!(f, "{:#x}: {}", self.ptr as usize + i, pte))
    }
}

const fn loc_to_slot(loc: usize) -> usize {
    (loc >> 30) & 0b111111111
}

pub fn init(
    root_page_table_ptr: *const (),
    phys_base_loc: usize,
    virt_base_loc: usize,
) -> PageTable<PageTableLevelRoot> {
    let mut root_page_table = PageTable::new_root(root_page_table_ptr, false);

    let identity_slot = loc_to_slot(phys_base_loc);
    let high_half_slot = loc_to_slot(virt_base_loc);
    let kernel_pte = PageTableEntry::leaf(phys_base_loc as *const ());
    root_page_table.set_pte(identity_slot, kernel_pte);
    root_page_table.set_pte(high_half_slot, kernel_pte);

    let uart_slot = loc_to_slot(VIRT_BASE + crate::uart::Uart::OFFSET);
    let uart_pte = PageTableEntry::leaf(0x00000000 as *const ());
    root_page_table.set_pte(uart_slot, uart_pte);

    // pre-locate the first pte, to avoid null-pointer dereference
    let empty_slot = loc_to_slot(0x00000000);
    let empty_pte = PageTableEntry::leaf(0x00000000 as *const ());
    root_page_table.set_pte(empty_slot, empty_pte);

    root_page_table
}

pub fn remove_kernel_identity_map(root_page_table: &mut PageTable<PageTableLevelRoot>) {
    let identity_slot = loc_to_slot(KERNEL_OFFSET);

    let empty_pte = PageTableEntry::empty();
    root_page_table.set_pte(identity_slot, empty_pte);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_loc_to_slot() {
        assert_eq!(loc_to_slot(0x00000000), 0);
        assert_eq!(loc_to_slot(0x40000000), 1);
        assert_eq!(loc_to_slot(KERNEL_OFFSET), 2);
        assert_eq!(loc_to_slot(VIRT_BASE), 0x1FC);
    }

    #[test_case]
    fn test_page_table_new() {
        let page_ptr = 0x80000000 as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new(page_ptr, 0);

        // Verify that the page table was created by testing SATP value
        let satp = page_table.satp();
        let ppn = satp & 0xFFFFFFFFFFF;
        assert_eq!(ppn, 0x80000, "PPN in SATP should match page pointer");
    }

    #[test_case]
    fn test_page_table_new_root_physical() {
        let page_ptr = 0x80000000 as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, false);

        // Verify creation by checking SATP value
        let satp = page_table.satp();
        let ppn = satp & 0xFFFFFFFFFFF;
        assert_eq!(ppn, 0x80000, "PPN should match page pointer");

        // Verify physical offset by converting a pointer
        let test_ptr = 0x1000 as *const ();
        let converted = page_table.page_table_ptr(test_ptr);
        assert_eq!(
            converted as usize,
            PHYS_BASE + 0x1000,
            "should use physical base offset"
        );
    }

    #[test_case]
    fn test_page_table_new_root_virtual() {
        let page_ptr = 0x80000000 as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, true);

        // Verify creation by checking SATP value
        let satp = page_table.satp();
        let ppn = satp & 0xFFFFFFFFFFF;
        assert_eq!(ppn, 0x80000, "PPN should match page pointer");

        // Verify virtual offset by converting a pointer
        let test_ptr = 0x1000 as *const ();
        let converted = page_table.page_table_ptr(test_ptr);
        assert_eq!(
            converted as usize,
            VIRT_BASE + 0x1000,
            "should use virtual base offset"
        );
    }

    #[test_case]
    fn test_satp_calculation() {
        let page_ptr = 0x80001000 as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, false);

        let satp = page_table.satp();

        // SATP format for Sv39: MODE (bits 63:60) = 8, PPN (bits 43:0)
        let mode = (satp >> 60) & 0xF;
        let ppn = satp & 0xFFFFFFFFFFF;

        assert_eq!(mode, 0b1000, "MODE field should be 8 for Sv39");
        assert_eq!(ppn, 0x80001, "PPN should match the page pointer");
    }

    #[test_case]
    fn test_page_table_get_set_pte() {
        // Allocate a page for the page table
        let mut page_table_storage = [PageTableEntry::empty(); 512];
        let page_ptr = page_table_storage.as_mut_ptr() as *const ();
        let mut page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, false);

        // Initially should be empty/invalid
        let initial_pte = page_table.get_pte(0);
        assert!(!initial_pte.is_valid(), "initial PTE should be invalid");

        // Set a PTE
        let new_pte = PageTableEntry::leaf(0x80000000 as *const ());
        page_table.set_pte(0, new_pte);

        // Read it back
        let read_pte = page_table.get_pte(0);
        assert!(read_pte.is_valid(), "PTE should now be valid");
        assert_eq!(read_pte.0, new_pte.0, "PTE value should match what was set");
    }

    #[test_case]
    fn test_page_table_free_index() {
        let mut page_table_storage = [PageTableEntry::empty(); 512];
        let page_ptr = page_table_storage.as_mut_ptr() as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, false);

        // All entries are empty, so first free should be 0
        let free = page_table.free_index();
        assert_eq!(free, Some(0), "first free index should be 0");
    }

    #[test_case]
    fn test_page_table_free_index_partial() {
        let mut page_table_storage = [PageTableEntry::empty(); 512];
        let page_ptr = page_table_storage.as_mut_ptr() as *const ();
        let mut page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, false);

        // Fill first few entries
        for i in 0..3 {
            page_table.set_pte(i, PageTableEntry::leaf(0x80000000 as *const ()));
        }

        // Next free should be 3
        let free = page_table.free_index();
        assert_eq!(free, Some(3), "first free index should be 3");
    }

    #[test_case]
    fn test_page_table_get_ptes_iterator() {
        let mut page_table_storage = [PageTableEntry::empty(); 512];
        let page_ptr = page_table_storage.as_mut_ptr() as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, false);

        // Count the entries without allocating
        let count = page_table.get_ptes().count();
        assert_eq!(count, 512, "should iterate over all 512 entries");
    }

    #[test_case]
    fn test_page_table_ptr_conversion() {
        let page_ptr = 0x80000000 as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, false);

        let phys_ptr = 0x80001000 as *const ();
        let virt_ptr = page_table.page_table_ptr(phys_ptr);

        // With PHYS_BASE offset, should be the same
        assert_eq!(
            virt_ptr as usize,
            PHYS_BASE + phys_ptr as usize,
            "should apply offset correctly"
        );
    }

    #[test_case]
    fn test_page_table_ptr_conversion_virtual() {
        let page_ptr = 0x80000000 as *const ();
        let page_table = PageTable::<PageTableLevelRoot>::new_root(page_ptr, true);

        let phys_ptr = 0x80001000 as *const ();
        let virt_ptr = page_table.page_table_ptr(phys_ptr);

        // With VIRT_BASE offset, should add the high-half offset
        assert_eq!(
            virt_ptr as usize,
            VIRT_BASE + phys_ptr as usize,
            "should apply virtual offset correctly"
        );
    }
}
