pub struct Pte(pub u64);
impl Pte {
    pub fn new(page_ptr: *const (), attributes: PteAttributes) -> Self {
        let reserved = 0b0;
        let ppn = Self::pnn(page_ptr);
        let rsw = 0b00 << 8;
        let attributes = attributes.0 as u64;
        Self(reserved | ppn | rsw | attributes)
    }

    fn pnn(page_ptr: *const ()) -> u64 {
        (page_ptr as u64 >> 12) << 10
    }

    fn attributes(
        dirty: bool,
        accessed: bool,
        global: bool,
        user: bool,
        execute: bool,
        write: bool,
        read: bool,
    ) -> u8 {
        let f = |c| if c { 0b1 } else { 0b0 };

        // Always valid
        0b1 | (f(dirty) << 7)
            | (f(accessed) << 6)
            | (f(global) << 5)
            | (f(user) << 4)
            | (f(execute) << 3)
            | (f(write) << 2)
            | (f(read) << 1)
    }
}

#[derive(Copy, Clone)]
pub struct PteAttributes(u8);
impl PteAttributes {
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
impl Default for PteAttributes {
    fn default() -> Self {
        Self(0b1)
    }
}
