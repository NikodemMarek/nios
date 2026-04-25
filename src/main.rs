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
use crate::memory_manager::{MemoryManager, Pmm, Vmm, read_setup_page, write_setup_page};

core::arch::global_asm!(include_str!("bootloader.s"));

const PHYS_BASE: usize = 0x00000000;
const VIRT_BASE: usize = 0xffffffff00000000;
const KERNEL_OFFSET: usize = 0x80200000;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text")]
pub extern "C" fn kernel_main() {
    writeln!(crate::sbi::Sbi, "Hello from high-half!");

    loop {}
}

#[global_allocator]
static ALLOCATOR: GlobalAllocator<Vmm> = GlobalAllocator::empty();

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text")]
pub extern "C" fn kernel_main_virtual() -> ! {
    // runs at virtual address, after MMU is on

    // Update trap vector to virtual address
    unsafe extern "C" {
        fn trap_entry();
    }
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) trap_entry);
    }

    let setup_page_loc: usize;
    unsafe {
        core::arch::asm!("mv {setup_page_ptr}, t5", setup_page_ptr = out(reg) setup_page_loc);
    }

    let (pmm, root_page_table) = read_setup_page(setup_page_loc);
    let vmm = Vmm::new(pmm, root_page_table);

    // TODO: Remove identity mapping

    let heap = Heap::new(vmm);
    ALLOCATOR.init(heap);

    shell::run();

    loop {}
}

#[cfg(test)]
mod test;
