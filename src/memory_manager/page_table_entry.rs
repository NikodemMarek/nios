pub struct PageTableEntry(pub u64);
impl PageTableEntry {
    pub fn new(page_ptr: *const (), attributes: PageTableEntryAttributes) -> Self {
        let reserved = 0b0;
        let ppn = Self::pnn(page_ptr);
        let rsw = 0b00 << 8;
        let attributes = attributes.0 as u64;
        Self(reserved | ppn | rsw | attributes)
    }

    pub fn from_ptr(ptr: *const u64) -> Self {
        PageTableEntry(if ptr.is_null() { 0b0 } else { unsafe { *ptr } })
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
