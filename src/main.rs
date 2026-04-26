#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod global_allocator;
mod heap;
mod memory_manager;
mod panic;
mod qemu;
mod sbi;
mod shell;
mod traps;
mod uart;

use core::fmt::Write;

use crate::global_allocator::GlobalAllocator;
use crate::heap::Heap;
use crate::memory_manager::{PageTable, Vmm};

core::arch::global_asm!(include_str!("bootloader.s"));

#[global_allocator]
static ALLOCATOR: GlobalAllocator<Vmm> = GlobalAllocator::empty();

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text")]
pub extern "C" fn kernel_main() {
    const MEMORY_SIZE: usize = 128 * 1024 * 1024;

    unsafe extern "C" {
        fn trap_entry();
    }
    unsafe {
        core::arch::asm!(
            "csrw stvec, {0}",
            "csrw sscratch, zero",
            in(reg) trap_entry as usize,
        );
    }

    writeln!(crate::sbi::Sbi, "Hello from high-half!");

    let mut pmm = memory_manager::init_pmm(MEMORY_SIZE);

    unsafe extern "C" {
        static _root_page_table_virt: u8;
    }
    let root_page_table_ptr = unsafe { &_root_page_table_virt } as *const u8;
    let mut root_page_table = PageTable::new_root(root_page_table_ptr as *const ());
    root_page_table.add_page(&mut pmm); // reserve page starting at 0x0 because it will produce null-pointer

    let vmm = Vmm::new(pmm, root_page_table);
    let heap = Heap::new(vmm);

    ALLOCATOR.init(heap);

    let mut incount = 0;
    let mut i = || -> u8 {
        if incount > 50 {
            loop {}
        }
        incount += 1;
        50
    };
    shell::run(&mut i, &mut sbi::Sbi);

    loop {}
}

#[cfg(test)]
mod test;
