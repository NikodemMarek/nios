#[repr(C)]
#[derive(Copy, Clone)]
pub struct PageTableEntry(pub u64);
impl PageTableEntry {
    pub fn new(page_ptr: *const (), attributes: PageTableEntryAttributes) -> Self {
        let reserved = 0b0;
        let ppn = Self::pnn(page_ptr);
        let rsw = 0b00 << 8;
        let attributes = attributes.0 as u64;
        Self(reserved | ppn | rsw | attributes)
    }
    pub fn page_table(page_ptr: *const ()) -> Self {
        Self::new(page_ptr, PageTableEntryAttributes::page_table())
    }
    pub fn leaf(page_ptr: *const ()) -> Self {
        Self::new(page_ptr, PageTableEntryAttributes::leaf())
    }
    pub fn empty() -> Self {
        Self(0b0)
    }

    pub fn from_ptr(ptr: *const PageTableEntry) -> Self {
        if ptr.is_null() {
            PageTableEntry(0b0)
        } else {
            unsafe { *ptr }
        }
    }

    fn pnn(page_ptr: *const ()) -> u64 {
        (page_ptr as u64 >> 12) << 10
    }

    pub fn page_ptr(&self) -> *const () {
        let page_loc = (self.0 >> 10) << 12;
        page_loc as *const ()
    }

    pub fn is_valid(&self) -> bool {
        (self.0 & 0b1) == 0b1
    }

    pub fn is_leaf(&self) -> bool {
        ((self.0 & 0b1110) >> 1) != 0
    }
}
impl core::fmt::Display for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:064b}", self.0)
    }
}

#[derive(Copy, Clone)]
pub struct PageTableEntryAttributes(u8);
impl PageTableEntryAttributes {
    fn page_table() -> Self {
        Self::default()
    }
    fn leaf() -> Self {
        Self::default().dirty().accessed().execute().write().read()
    }

    pub fn dirty(mut self) -> Self {
        self.0 |= 0b1 << 7;
        self
    }
    pub fn accessed(mut self) -> Self {
        self.0 |= 0b1 << 6;
        self
    }
    pub fn global(mut self) -> Self {
        self.0 |= 0b1 << 5;
        self
    }
    pub fn user(mut self) -> Self {
        self.0 |= 0b1 << 4;
        self
    }
    pub fn execute(mut self) -> Self {
        self.0 |= 0b1 << 3;
        self
    }
    pub fn write(mut self) -> Self {
        self.0 |= 0b1 << 2;
        self
    }
    pub fn read(mut self) -> Self {
        self.0 |= 0b1 << 1;
        self
    }
}
impl Default for PageTableEntryAttributes {
    fn default() -> Self {
        // always valid
        Self(0b1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_empty_pte() {
        let pte = PageTableEntry::empty();
        assert!(!pte.is_valid(), "empty PTE should not be valid");
        assert!(!pte.is_leaf(), "empty PTE should not be a leaf");
    }

    #[test_case]
    fn test_page_table_pte() {
        let page_ptr = 0x80001000 as *const ();
        let pte = PageTableEntry::page_table(page_ptr);

        assert!(pte.is_valid(), "page table PTE should be valid");
        assert!(!pte.is_leaf(), "page table PTE should not be a leaf");
        assert_eq!(pte.page_ptr(), page_ptr, "should preserve page pointer");
    }

    #[test_case]
    fn test_leaf_pte() {
        let page_ptr = 0x80002000 as *const ();
        let pte = PageTableEntry::leaf(page_ptr);

        assert!(pte.is_valid(), "leaf PTE should be valid");
        assert!(pte.is_leaf(), "leaf PTE should be a leaf");
        assert_eq!(pte.page_ptr(), page_ptr, "should preserve page pointer");
    }

    #[test_case]
    fn test_pte_from_null_ptr() {
        let pte = PageTableEntry::from_ptr(core::ptr::null());
        assert!(!pte.is_valid(), "PTE from null pointer should not be valid");
        assert_eq!(pte.0, 0, "PTE from null pointer should be zero");
    }

    #[test_case]
    fn test_page_ptr_roundtrip() {
        // Test various page-aligned addresses
        let addresses = [
            0x80000000 as *const (),
            0x80001000 as *const (),
            0x80FFF000 as *const (),
            0xFFFFFFFF00000000 as *const (),
        ];

        for &addr in &addresses {
            let pte = PageTableEntry::page_table(addr);
            assert_eq!(
                pte.page_ptr(),
                addr,
                "page pointer should roundtrip correctly for {:p}",
                addr
            );
        }
    }

    #[test_case]
    fn test_pte_ppn_calculation() {
        // Physical page number (PPN) should be bits [53:10] in the PTE
        // For address 0x80001000, PPN should be 0x80001
        let page_ptr = 0x80001000 as *const ();
        let pte = PageTableEntry::page_table(page_ptr);

        // Extract PPN from the PTE (bits [53:10])
        let ppn = (pte.0 >> 10) & 0x3FFFFFFFFFFF;
        assert_eq!(ppn, 0x80001, "PPN should be correctly calculated");
    }

    #[test_case]
    fn test_pte_valid_bit() {
        let page_ptr = 0x80000000 as *const ();

        // Page table entry should have valid bit set
        let pte = PageTableEntry::page_table(page_ptr);
        assert_eq!(pte.0 & 0b1, 1, "valid bit should be set for page table");

        // Leaf entry should also have valid bit set
        let leaf_pte = PageTableEntry::leaf(page_ptr);
        assert_eq!(leaf_pte.0 & 0b1, 1, "valid bit should be set for leaf");

        // Empty entry should not have valid bit set
        let empty_pte = PageTableEntry::empty();
        assert_eq!(
            empty_pte.0 & 0b1,
            0,
            "valid bit should not be set for empty"
        );
    }

    #[test_case]
    fn test_leaf_rwx_bits() {
        let page_ptr = 0x80000000 as *const ();
        let pte = PageTableEntry::leaf(page_ptr);

        // Leaf entries should have R, W, X bits set (bits 1, 2, 3)
        let rwx_bits = (pte.0 >> 1) & 0b111;
        assert_ne!(rwx_bits, 0, "leaf should have at least one of R/W/X set");
        assert!(pte.is_leaf(), "entry with R/W/X should be a leaf");
    }

    #[test_case]
    fn test_attributes_builder() {
        let attrs = PageTableEntryAttributes::default().read().write().execute();

        // Check that the bits are set correctly
        assert_eq!(attrs.0 & 0b1, 1, "valid bit should be set");
        assert_eq!((attrs.0 >> 1) & 0b1, 1, "read bit should be set");
        assert_eq!((attrs.0 >> 2) & 0b1, 1, "write bit should be set");
        assert_eq!((attrs.0 >> 3) & 0b1, 1, "execute bit should be set");
    }

    #[test_case]
    fn test_attributes_all_flags() {
        let attrs = PageTableEntryAttributes::default()
            .read()
            .write()
            .execute()
            .user()
            .global()
            .accessed()
            .dirty();

        // Verify all bits are set
        assert_eq!(attrs.0 & 0b1, 1, "valid bit");
        assert_eq!((attrs.0 >> 1) & 0b1, 1, "read bit");
        assert_eq!((attrs.0 >> 2) & 0b1, 1, "write bit");
        assert_eq!((attrs.0 >> 3) & 0b1, 1, "execute bit");
        assert_eq!((attrs.0 >> 4) & 0b1, 1, "user bit");
        assert_eq!((attrs.0 >> 5) & 0b1, 1, "global bit");
        assert_eq!((attrs.0 >> 6) & 0b1, 1, "accessed bit");
        assert_eq!((attrs.0 >> 7) & 0b1, 1, "dirty bit");
    }
}
