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

use crate::global_allocator::GlobalAllocator;
use crate::heap::Heap;
use crate::memory_manager::{MemoryManager, Pmm, Vmm, read_setup_page, write_setup_page};

core::arch::global_asm!(include_str!("main.s"));

const PHYS_BASE: usize = 0x00000000;
const VIRT_BASE: usize = 0xffffffff00000000;
const KERNEL_OFFSET: usize = 0x80200000;

#[inline(always)]
fn sbi_call(
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    fid: usize,
    eid: usize,
) -> (isize, isize) {
    let (error, value);
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") eid,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    (error, value)
}

fn putchar(c: char) {
    let buffer = [c as u8];
    let ptr = buffer.as_ptr() as usize;
    sbi_call(1, ptr, 0, 0, 0, 0, 0, 0x4442434E);
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() {
    for c in "Hello nios!".chars() {
        putchar(c);
    }

    loop {}
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

    let current_time: u64;
    unsafe {
        core::arch::asm!("csrr {res}, time", res = out(reg) current_time);
        scheduler::set_timer(current_time + 10_000_000);
    }

    shell::run();

    loop {}
}

#[cfg(test)]
mod test;
