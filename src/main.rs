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
mod shell;
mod traps;
mod uart;

use core::arch::global_asm;

use crate::global_allocator::GlobalAllocator;
use crate::heap::Heap;
use crate::memory_manager::{MemoryManager, Pmm, Vmm, read_setup_page, write_setup_page};

global_asm!(include_str!("main.s"));

const PHYS_BASE: usize = 0x00000000;
const VIRT_BASE: usize = 0xffffffff00000000;
const KERNEL_OFFSET: usize = 0x80000000;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() {
    if cfg!(test) {
        #[cfg(test)]
        test_main();

        qemu::exit(qemu::ExitCode::Success);
    } else {
        // runs at physical address, before MMU
        enable_virtual_memory(); // noreturn, jumps to kernel_main_virtual
    }
}

#[unsafe(link_section = ".text.boot")]
pub fn enable_virtual_memory() {
    let phys_base_loc = PHYS_BASE + KERNEL_OFFSET;
    let virt_base_loc = VIRT_BASE + KERNEL_OFFSET;

    let mut pmm = Pmm::init();

    let root_page_table_ptr = pmm.alloc().expect("PMM out of pages");
    let root_page_table = memory_manager::init_page_table(
        root_page_table_ptr as *const (),
        phys_base_loc,
        virt_base_loc,
    );
    let satp_val = root_page_table.satp();

    // write info required to load the setup after virtual memory is enabled
    let setup_page_loc = write_setup_page(&mut pmm, root_page_table_ptr as *const ());

    let phys_entry = kernel_main_virtual as *const () as usize;
    let virt_entry = (phys_entry - phys_base_loc + virt_base_loc) as *const ();

    unsafe {
        core::arch::asm!(
            "csrw satp, {satp_val}",
            "sfence.vma zero, zero",
            "mv t5, {setup_page_ptr}",
            "li t0, 0xffffffff00000000",
            "add sp, sp, t0",
            "jr {v_addr}",
            satp_val = in(reg) satp_val,
            setup_page_ptr = in(reg) setup_page_loc,
            v_addr = in(reg) virt_entry,
            options(noreturn)
        );
    }
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
