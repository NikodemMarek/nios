mod page_table;
mod page_table_entry;
mod pmm;
mod vmm;

pub use page_table::{PageTable, create_page_table, init_page_table};
pub use pmm::{Pmm, init_pmm};
pub use vmm::Vmm;

pub const PAGE_SIZE: usize = 4096;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PhysicalAddress(pub usize);
impl PhysicalAddress {
    pub fn virt(&self) -> VirtualAddress {
        VirtualAddress(self.0 + 0xffffffff00000000)
    }
}
impl<T> From<*const T> for PhysicalAddress {
    fn from(value: *const T) -> Self {
        Self(value as usize)
    }
}
impl From<usize> for PhysicalAddress {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VirtualAddress(pub usize);
impl VirtualAddress {
    pub fn new_sv39_page(l2: usize, l1: usize, l0: usize) -> Self {
        Self(Self::sv39_pad((l2 << 30) | (l1 << 21) | (l0 << 12)))
    }
    pub fn new_sv39_megapage(l2: usize, l1: usize) -> Self {
        Self(Self::sv39_pad((l2 << 30) | (l1 << 21)))
    }
    pub fn new_sv39_gigapage(l2: usize) -> Self {
        Self(Self::sv39_pad(l2 << 30))
    }
    fn sv39_pad(virtual_address: usize) -> usize {
        ((virtual_address as i64) << 25 >> 25) as usize
    }

    pub fn sv39_l2_index(&self) -> usize {
        (self.0 >> 30) & 0b111111111
    }
    pub fn sv39_l1_index(&self) -> usize {
        (self.0 >> 21) & 0b111111111
    }
    pub fn sv39_l0_index(&self) -> usize {
        (self.0 >> 12) & 0b111111111
    }

    pub fn phys(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 - 0xffffffff00000000)
    }

    pub fn add(&self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}
impl<T> From<*const T> for VirtualAddress {
    fn from(value: *const T) -> Self {
        Self(value as usize)
    }
}
impl From<usize> for VirtualAddress {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[cfg(test)]
pub use pmm::tests::setup_test_pmm;
#[cfg(test)]
pub use vmm::tests::setup_test_vmm;

#[cfg(test)]
mod tests {
    use crate::memory_manager::{PhysicalAddress, VirtualAddress};

    #[test_case]
    fn test_get_phys_addr() {
        let virt: VirtualAddress = (0xffffffff00000000usize + 0x80001000).into();
        let phys = virt.phys();
        assert_eq!(phys.0, 0x80001000);
    }

    #[test_case]
    fn test_get_virt_addr() {
        let phys: PhysicalAddress = 0x80001000.into();
        let virt = phys.virt();
        assert_eq!(virt.0, 0xffffffff00000000usize + 0x80001000);
    }
}
