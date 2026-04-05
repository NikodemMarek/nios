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
use crate::memory_manager::{Pmm, Vmm};

global_asm!(include_str!("main.s"));

unsafe extern "C" {
    static PHYS_BASE: u8;
    static VIRT_BASE: u8;
}

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
    let phys_base_loc = unsafe { &PHYS_BASE as *const u8 as usize };
    // this does not work in code-model=medium
    // let virt_base = unsafe { &VIRT_BASE as *const u8 as usize };
    // so i just hardcode the same value here
    let virt_base_loc: usize = 0xffffffff80000000;

    let mut pmm = Pmm::init();

    let root_page_table = memory_manager::init_page_table(&mut pmm, phys_base_loc, virt_base_loc);
    let satp_val = root_page_table.satp();

    let vmm = Vmm::new(pmm, root_page_table);
    let vmm_ptr = &vmm as *const Vmm;

    let phys_entry = kernel_main_virtual as *const () as usize;
    let virt_entry = (phys_entry - phys_base_loc + virt_base_loc) as *const ();

    unsafe {
        core::arch::asm!(
            "csrw satp, {satp_val}",
            "sfence.vma zero, zero",
            "mv t5, {mm_ptr}",
            "jr {v_addr}",
            satp_val = in(reg) satp_val,
            mm_ptr = in(reg) vmm_ptr,
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
    println!("virtual memory enabled");

    let vmm_ptr: *const Vmm;
    unsafe {
        core::arch::asm!("mv {mm_ptr}, t5", mm_ptr = out(reg) vmm_ptr);
    }
    let vmm = unsafe { *vmm_ptr } as Vmm;

    let heap = Heap::new(vmm);
    ALLOCATOR.init(heap);

    shell::run();

    loop {}
}

#[cfg(test)]
mod test;
