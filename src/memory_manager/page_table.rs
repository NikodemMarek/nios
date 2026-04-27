use crate::memory_manager::{MemoryManager, Pmm, page_table_entry::PageTableEntry};

#[derive(Copy, Clone)]
pub struct PageTableLevelRoot;
#[derive(Copy, Clone)]
struct PageTableLevelL1;
#[derive(Copy, Clone)]
struct PageTableLevelL0;

pub(crate) trait PageTableHasChildren {}
impl PageTableHasChildren for PageTableLevelRoot {}
impl PageTableHasChildren for PageTableLevelL1 {}

#[derive(Copy, Clone)]
pub struct PageTable<PageTableLevel = PageTableLevelRoot> {
    level: core::marker::PhantomData<PageTableLevel>,
    ptr: *const PageTableEntry,
}

impl<L> PageTable<L> {
    fn existing(ptr: *const ()) -> Self {
        Self {
            level: core::marker::PhantomData,
            ptr: ptr as *const PageTableEntry,
        }
    }
    fn new(ptr: *const ()) -> Self {
        let ptr = ptr as *mut PageTableEntry;
        for i in 0..512 {
            unsafe { *ptr.add(i) = PageTableEntry::empty() };
        }

        Self {
            level: core::marker::PhantomData,
            ptr,
        }
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
    fn get_free_index(&self) -> Option<usize> {
        self.get_ptes()
            .enumerate()
            .find(|(_, pte)| !pte.is_valid())
            .map(|(i, _)| i)
    }

    fn is_full(&self) -> bool {
        self.get_ptes().all(|pte| pte.is_valid())
    }

    fn add_leaf(&mut self, pmm: &mut super::Pmm) -> Option<usize> {
        let free_index = self.get_free_index()?;

        let leaf_page_ptr = pmm.alloc().expect("PMM out of pages");
        let leaf_page_ptr = get_phys_ptr(leaf_page_ptr);
        let leaf_page_pte = PageTableEntry::leaf(leaf_page_ptr);

        self.set_pte(free_index, leaf_page_pte);

        Some(free_index)
    }
}
impl<L: PageTableHasChildren> PageTable<L> {
    fn get_empty_or_non_full_child_page_table<C>(
        &mut self,
        pmm: &mut Pmm,
    ) -> Option<(usize, PageTable<C>)> {
        enum Found {
            Empty(usize),
            NotFull(usize, *const ()),
        }

        let found = self
            .get_ptes()
            .enumerate()
            .filter(|(_, pte)| !pte.is_leaf())
            .find_map(|(index, pte)| {
                if !pte.is_valid() {
                    return Some(Found::Empty(index));
                }
                let ptr = get_virt_ptr(pte.page_ptr());
                let page_table = PageTable::<C>::existing(ptr);
                (!page_table.is_full()).then_some(Found::NotFull(index, ptr))
            })?;

        match found {
            Found::Empty(index) => Some(self.set_page_table(pmm, index)),
            Found::NotFull(index, ptr) => Some((index, PageTable::<C>::existing(ptr))),
        }
    }
    fn add_page_table<C>(&mut self, pmm: &mut Pmm) -> Option<(usize, PageTable<C>)> {
        let free_index = self.get_free_index()?;

        let page_table_ptr = pmm.alloc().expect("PMM out of pages");
        let page_table_ptr = get_phys_ptr(page_table_ptr);
        let page_table_pte = PageTableEntry::page_table(page_table_ptr);

        self.set_pte(free_index, page_table_pte);

        let page_table = PageTable::<C>::new(page_table_ptr);
        Some((free_index, page_table))
    }
    fn set_page_table<C>(&mut self, pmm: &mut Pmm, index: usize) -> (usize, PageTable<C>) {
        let page_table_ptr = pmm.alloc().expect("PMM out of pages");
        let page_table_ptr = get_phys_ptr(page_table_ptr);
        let page_table_pte = PageTableEntry::page_table(page_table_ptr);

        self.set_pte(index, page_table_pte);

        let page_table = PageTable::<C>::new(page_table_ptr);
        (index, page_table)
    }
}

impl PageTable<PageTableLevelRoot> {
    pub fn new_root(ptr: *const ()) -> PageTable<PageTableLevelRoot> {
        PageTable {
            level: core::marker::PhantomData::<PageTableLevelRoot>,
            ptr: ptr as *const PageTableEntry,
        }
    }

    pub fn satp(&self) -> u64 {
        let ppn = (self.ptr as u64) >> 12;
        (0b1000u64 << 60) | ppn
    }

    pub fn add_gigapage(&mut self, pmm: &mut super::Pmm) -> Option<usize> {
        self.add_leaf(pmm)
    }
    pub fn add_megapage(&mut self, pmm: &mut super::Pmm) -> Option<(usize, usize)> {
        let (l2_index, mut l1_page_table) =
            self.get_empty_or_non_full_child_page_table::<PageTableLevelL1>(pmm)?;
        let l1_index = l1_page_table.add_megapage(pmm)?;
        Some((l2_index, l1_index))
    }
    pub fn add_page(&mut self, pmm: &mut super::Pmm) -> Option<(usize, usize, usize)> {
        let (l2_index, mut l1_page_table) =
            self.get_empty_or_non_full_child_page_table::<PageTableLevelL1>(pmm)?;
        let (l1_index, l0_index) = l1_page_table.add_page(pmm)?;
        Some((l2_index, l1_index, l0_index))
    }
}
impl PageTable<PageTableLevelL1> {
    pub fn add_megapage(&mut self, pmm: &mut super::Pmm) -> Option<usize> {
        self.add_leaf(pmm)
    }
    pub fn add_page(&mut self, pmm: &mut super::Pmm) -> Option<(usize, usize)> {
        let (l1_index, mut l0_page_table) =
            self.get_empty_or_non_full_child_page_table::<PageTableLevelL0>(pmm)?;
        let l0_index = l0_page_table.add_page(pmm)?;
        Some((l1_index, l0_index))
    }
}
impl PageTable<PageTableLevelL0> {
    pub fn add_page(&mut self, pmm: &mut super::Pmm) -> Option<usize> {
        self.add_leaf(pmm)
    }
}
impl<L> core::fmt::Display for PageTable<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.get_ptes()
            .enumerate()
            .try_for_each(|(i, pte)| writeln!(f, "{:#x}: {}", self.ptr as usize + i, pte))
    }
}

fn get_phys_ptr(ptr: *const ()) -> *const () {
    (ptr as usize - 0xffffffff00000000) as *const ()
}
fn get_virt_ptr(page_ptr: *const ()) -> *const () {
    (page_ptr as usize + 0xffffffff00000000) as *const ()
}

pub fn init_page_table(pmm: &mut Pmm) -> PageTable<PageTableLevelRoot> {
    unsafe extern "C" {
        static _root_page_table_virt: u8;
    }

    let root_page_table_ptr = unsafe { &_root_page_table_virt } as *const u8;
    let mut root_page_table = PageTable::new_root(root_page_table_ptr as *const ());
    root_page_table.add_page(pmm); // reserve page starting at 0x0 because it will produce null-pointer
    root_page_table
}

// const fn loc_to_slot(loc: usize) -> usize {
//     (loc >> 30) & 0b111111111
// }
//
// pub fn remove_kernel_identity_map(root_page_table: &mut PageTable<PageTableLevelRoot>) {
//     let identity_slot = loc_to_slot(KERNEL_OFFSET);
//
//     let empty_pte = PageTableEntry::empty();
//     root_page_table.set_pte(identity_slot, empty_pte);
// }

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
        let page_table = PageTable::<PageTableLevelRoot>::existing(page_ptr, 0);

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
