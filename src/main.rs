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
use crate::memory_manager::{MemoryManager, Pmm, Pte, PteAttributes, Vmm, satp};

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
        let mut pmm = Pmm::init();
        enable_virtual_memory(pmm); // noreturn, jumps to kernel_main_virtual
    }
}

#[unsafe(link_section = ".text.boot")]
pub fn enable_virtual_memory(mut pmm: Pmm) {
    let phys_base_loc = unsafe { &PHYS_BASE as *const u8 as usize };
    // this does not work in code-model=medium
    // let virt_base = unsafe { &VIRT_BASE as *const u8 as usize };
    // so i just hardcode the same value here
    let virt_base_loc: usize = 0xffffffff80000000;

    let phys_root_ptr = pmm.alloc().expect("PMM out of pages") as *mut u64;

    const fn loc_to_slot(loc: usize) -> usize {
        (loc >> 30) & 0b111111111
    }
    let identity_slot = loc_to_slot(phys_base_loc);
    let high_half_slot = loc_to_slot(virt_base_loc);
    let uart_slot = loc_to_slot(0x00000000);

    let pte_attrs = PteAttributes::default()
        .dirty()
        .accessed()
        .execute()
        .write()
        .read();
    let kernel_pte = Pte::new(phys_base_loc as *const (), pte_attrs).0;
    let uart_pte = Pte::new(0x00000000 as *const (), pte_attrs).0;

    unsafe {
        let identity_slot_ptr = phys_root_ptr.add(identity_slot);
        let high_half_slot_ptr = phys_root_ptr.add(high_half_slot);
        let uart_slot_ptr = phys_root_ptr.add(uart_slot);
        *identity_slot_ptr = kernel_pte;
        *high_half_slot_ptr = kernel_pte;
        *uart_slot_ptr = uart_pte;
    }

    let satp_val = satp(phys_root_ptr as *mut ());

    let vmm = Vmm::init(pmm, phys_root_ptr as *const ());
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
