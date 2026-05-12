use crate::memory_manager::{
    PhysicalAddress, Pmm, VirtualAddress, page_table_entry::PageTableEntry,
};

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
    addr: VirtualAddress,
}

impl<L> PageTable<L> {
    fn existing(addr: VirtualAddress) -> Self {
        Self {
            level: core::marker::PhantomData,
            addr,
        }
    }
    fn new(addr: VirtualAddress) -> Self {
        let ptr = addr.0 as *mut PageTableEntry;
        for i in 0..512 {
            unsafe { *ptr.add(i) = PageTableEntry::empty() };
        }

        Self {
            level: core::marker::PhantomData,
            addr,
        }
    }

    fn get_pte(&self, index: usize) -> PageTableEntry {
        assert!(index < 512, "page table entry index {index} too high");
        PageTableEntry::from_ptr(unsafe { (self.addr.0 as *const PageTableEntry).add(index) })
    }
    fn set_pte(&mut self, index: usize, pte: PageTableEntry) {
        unsafe {
            let page_pte_ptr =
                (self.addr.0 as *const PageTableEntry).add(index) as *mut PageTableEntry;
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

    fn add_leaf(&mut self, pmm: &mut Pmm) -> Option<usize> {
        let free_index = self.get_free_index()?;

        let leaf_page_addr = pmm.alloc().expect("PMM out of pages").into();
        let leaf_page_pte = PageTableEntry::leaf(get_phys_addr(leaf_page_addr));

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
            NotFull(usize, VirtualAddress),
        }

        let found = self
            .get_ptes()
            .enumerate()
            .filter(|(_, pte)| !pte.is_leaf())
            .find_map(|(index, pte)| {
                if !pte.is_valid() {
                    return Some(Found::Empty(index));
                }
                let addr = get_virt_addr(pte.page_ptr());
                let page_table = PageTable::<C>::existing(addr);
                (!page_table.is_full()).then_some(Found::NotFull(index, addr))
            })?;

        match found {
            Found::Empty(index) => Some((index, self.set_page_table(pmm, index))),
            Found::NotFull(index, ptr) => Some((index, PageTable::<C>::existing(ptr))),
        }
    }
    fn set_page_table<C>(&mut self, pmm: &mut Pmm, index: usize) -> PageTable<C> {
        let page_table_addr = VirtualAddress(pmm.alloc().expect("PMM out of pages") as usize);

        let page_table_pte = PageTableEntry::page_table(get_phys_addr(page_table_addr));
        self.set_pte(index, page_table_pte);

        PageTable::<C>::new(page_table_addr)
    }
}

impl PageTable<PageTableLevelRoot> {
    pub fn new_root(addr: VirtualAddress) -> PageTable<PageTableLevelRoot> {
        PageTable {
            level: core::marker::PhantomData::<PageTableLevelRoot>,
            addr,
        }
    }

    pub fn satp(&self) -> u64 {
        let ppn = (get_phys_addr(self.addr).0 as u64) >> 12;
        (0b1000u64 << 60) | ppn
    }

    pub fn add_gigapage(&mut self, pmm: &mut Pmm) -> Option<VirtualAddress> {
        Some(VirtualAddress::new_sv39_gigapage(self.add_leaf(pmm)?))
    }
    pub fn add_megapage(&mut self, pmm: &mut Pmm) -> Option<VirtualAddress> {
        let (l2_index, mut l1_page_table) =
            self.get_empty_or_non_full_child_page_table::<PageTableLevelL1>(pmm)?;
        let l1_index = l1_page_table.add_megapage(pmm)?;
        Some(VirtualAddress::new_sv39_megapage(l2_index, l1_index))
    }
    pub fn add_page(&mut self, pmm: &mut Pmm) -> Option<VirtualAddress> {
        let (l2_index, mut l1_page_table) =
            self.get_empty_or_non_full_child_page_table::<PageTableLevelL1>(pmm)?;
        let (l1_index, l0_index) = l1_page_table.add_page(pmm)?;
        Some(VirtualAddress::new_sv39_page(l2_index, l1_index, l0_index))
    }
}

impl PageTable<PageTableLevelL1> {
    fn add_megapage(&mut self, pmm: &mut Pmm) -> Option<usize> {
        self.add_leaf(pmm)
    }
    fn add_page(&mut self, pmm: &mut Pmm) -> Option<(usize, usize)> {
        let (l1_index, mut l0_page_table) =
            self.get_empty_or_non_full_child_page_table::<PageTableLevelL0>(pmm)?;
        let l0_index = l0_page_table.add_page(pmm)?;
        Some((l1_index, l0_index))
    }
}
impl PageTable<PageTableLevelL0> {
    fn add_page(&mut self, pmm: &mut Pmm) -> Option<usize> {
        self.add_leaf(pmm)
    }
}
impl<L> core::fmt::Display for PageTable<L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.get_ptes()
            .enumerate()
            .try_for_each(|(i, pte)| writeln!(f, "{:#x}: {}", self.addr.0 + i, pte))
    }
}

fn get_phys_addr(addr: VirtualAddress) -> PhysicalAddress {
    PhysicalAddress(addr.0 - 0xffffffff00000000)
}
fn get_virt_addr(addr: PhysicalAddress) -> VirtualAddress {
    VirtualAddress(addr.0 + 0xffffffff00000000)
}

pub fn init_page_table(pmm: &mut Pmm) -> PageTable<PageTableLevelRoot> {
    unsafe extern "C" {
        static _root_page_table_virt: u8;
    }

    let root_page_table_ptr = unsafe { &_root_page_table_virt } as *const u8;
    let mut root_page_table = PageTable::new_root(root_page_table_ptr.into());

    create_page_table(pmm, &mut root_page_table);

    unsafe {
        core::arch::asm!("sfence.vma zero, zero");
    }

    root_page_table
}
pub fn create_page_table(pmm: &mut Pmm, root_page_table: &mut PageTable<PageTableLevelRoot>) {
    root_page_table.add_page(pmm); // reserve page starting at 0x0 because it will produce null-pointer

    // create a gigapage mapping for kernel in higher-half
    let kernel_map_addr = VirtualAddress(0xffffffff80000000);
    root_page_table.set_pte(
        kernel_map_addr.sv39_l2_index(),
        PageTableEntry::leaf(PhysicalAddress(0x80000000)),
    );

    // create a megapage mapping for UART
    let uart_map_addr = VirtualAddress(0xffffffff10000000);
    let mut l1_page_table =
        root_page_table.set_page_table::<PageTableLevelL1>(pmm, uart_map_addr.sv39_l2_index());
    l1_page_table.set_pte(
        uart_map_addr.sv39_l1_index(),
        PageTableEntry::leaf(PhysicalAddress(0x10000000)),
    );

    // create a mapping for qemu
    let qemu_map_addr = VirtualAddress(0xffffffff00100000);
    let mut l0_page_table =
        l1_page_table.set_page_table::<PageTableLevelL0>(pmm, qemu_map_addr.sv39_l1_index());
    l0_page_table.set_pte(
        qemu_map_addr.sv39_l0_index(),
        PageTableEntry::leaf(PhysicalAddress(0x100000)),
    );

    // remove identity mapping
    let identity_kernel_map_addr = VirtualAddress(0x80000000);
    root_page_table.set_pte(
        identity_kernel_map_addr.sv39_l2_index(),
        PageTableEntry::empty(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_satp_calculation() {
        let page_table = PageTable::<PageTableLevelRoot>::new_root(0xffffffff80001000.into());

        let satp = page_table.satp();

        let mode = (satp >> 60) & 0xF;
        let ppn = satp & 0xFFFFFFFFFFF;

        assert_eq!(mode, 0b1000);
        assert_eq!(ppn, 0x80001);
    }

    #[test_case]
    fn test_page_table_get_set_pte() {
        // Allocate a page for the page table
        let page_table_storage = [PageTableEntry::empty(); 512];
        let mut page_table =
            PageTable::<PageTableLevelRoot>::new_root(page_table_storage.as_ptr().into());

        // Initially should be empty/invalid
        let initial_pte = page_table.get_pte(0);
        assert!(!initial_pte.is_valid(), "initial PTE should be invalid");

        // Set a PTE
        let new_pte = PageTableEntry::leaf(0x80000000.into());
        page_table.set_pte(0, new_pte);

        // Read it back
        let read_pte = page_table.get_pte(0);
        assert!(read_pte.is_valid(), "PTE should now be valid");
        assert_eq!(read_pte.0, new_pte.0, "PTE value should match what was set");
    }

    #[test_case]
    fn test_page_table_free_index() {
        let page_table_storage = [PageTableEntry::empty(); 512];
        let page_table =
            PageTable::<PageTableLevelRoot>::new_root(page_table_storage.as_ptr().into());

        // All entries are empty, so first free should be 0
        let free = page_table.get_free_index();
        assert_eq!(free, Some(0), "first free index should be 0");
    }

    #[test_case]
    fn test_page_table_free_index_partial() {
        let page_table_storage = [PageTableEntry::empty(); 512];
        let mut page_table =
            PageTable::<PageTableLevelRoot>::new_root(page_table_storage.as_ptr().into());

        // Fill first few entries
        for i in 0..3 {
            page_table.set_pte(i, PageTableEntry::leaf(0x80000000.into()));
        }

        // Next free should be 3
        let free = page_table.get_free_index();
        assert_eq!(free, Some(3), "first free index should be 3");
    }

    #[test_case]
    fn test_page_table_get_ptes_iterator() {
        let page_table_storage = [PageTableEntry::empty(); 512];
        let page_table =
            PageTable::<PageTableLevelRoot>::new_root(page_table_storage.as_ptr().into());

        // Count the entries without allocating
        let count = page_table.get_ptes().count();
        assert_eq!(count, 512, "should iterate over all 512 entries");
    }

    #[test_case]
    fn test_get_phys_addr() {
        let virt = (0xffffffff00000000usize + 0x80001000).into();
        let phys = get_phys_addr(virt);
        assert_eq!(phys.0, 0x80001000);
    }

    #[test_case]
    fn test_get_virt_addr() {
        let phys = 0x80001000.into();
        let virt = get_virt_addr(phys);
        assert_eq!(virt.0, 0xffffffff00000000usize + 0x80001000);
    }
}
