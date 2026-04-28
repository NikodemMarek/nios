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

use crate::global_allocator::GlobalAllocator;
use crate::heap::Heap;
use crate::memory_manager::Vmm;

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
        // set trap entry
        core::arch::asm!(
            "csrw stvec, {0}",
            "csrw sscratch, zero",
            in(reg) trap_entry as *const (),
        );
    }

    let mut pmm = memory_manager::init_pmm(MEMORY_SIZE);
    let root_page_table = memory_manager::init_page_table(&mut pmm);

    unsafe {
        // enable timer interrupts
        // clear a default interrupt set to 0 (fires immidiately)
        sbi::set_timer(u64::MAX);
        core::arch::asm!(
            "csrs sie, {stie}",
            "csrsi sstatus, 0x2",
            stie = in(reg) 0x20usize,
        );
    }

    if cfg!(test) {
        #[cfg(test)]
        test_main();

        qemu::exit(qemu::ExitCode::Success);
    } else {
        let vmm = Vmm::new(pmm, root_page_table);
        let heap = Heap::new(vmm);

        ALLOCATOR.init(heap);

        crate::sbi::set_timer(crate::sbi::read_time() + 50_000_000); // ~5s at 10MHz timebase

        shell::run(&mut crate::uart::Uart::read, &mut crate::uart::Uart);

        loop {}
    }
}

#[cfg(test)]
mod test;
