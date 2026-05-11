mod page_table;
mod page_table_entry;
mod pmm;
mod vmm;

pub use page_table::{PageTable, create_page_table, init_page_table};
pub use pmm::{Pmm, init_pmm};
pub use vmm::Vmm;

pub const PAGE_SIZE: usize = 4096;

#[derive(Copy, Clone)]
pub struct PhysicalAddress(usize);
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
#[derive(Copy, Clone)]
pub struct VirtualAddress(usize);
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
