#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod elf;
mod global_allocator;
mod memory_manager;
mod panic;
mod qemu;
mod sbi;
mod scheduler;
mod traps;
mod uart;

use core::cell::{RefCell, RefMut};
use nios_kernel_utils::heap::Heap;

use crate::global_allocator::GlobalAllocator;
use crate::memory_manager::{Pmm, Vmm};
use crate::scheduler::Scheduler;

core::arch::global_asm!(include_str!("bootloader.s"));

// TODO: Eventually load from a filesystem.
pub static SHELL_BINARY: &[u8] =
    include_bytes!("../../target/riscv64gc-unknown-none-elf/release/nios-shell");

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::empty();

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
        sbi::reset_timer();
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
        let heap = Heap::new(0xffffffff80300000);

        ALLOCATOR.init(heap);
        STATE.init(pmm);

        STATE.get().add(SHELL_BINARY);

        // start the interrupts immidiately
        crate::sbi::set_timer(crate::sbi::read_time() + 1_000_000);

        loop {}
    }
}

static STATE: KernelState = KernelState::empty();

struct KernelState(RefCell<Option<Scheduler>>);
impl KernelState {
    #[inline]
    pub const fn empty() -> Self {
        Self(RefCell::new(None))
    }
    #[inline]
    pub fn init(&self, pmm: Pmm) {
        *self.0.borrow_mut() = Some(Scheduler::new(pmm));
    }

    #[inline]
    fn get(&self) -> RefMut<'_, Scheduler> {
        RefMut::map(self.0.borrow_mut(), |mi| {
            mi.as_mut().expect("Kernel state not initialized")
        })
    }

    fn switch_task(&self, tf: &mut traps::TrapFrame) {
        let mut scheduler = self.get();

        scheduler.save(tf);

        let next_process = scheduler.next().expect("No process left to execute!");
        scheduler.restore(next_process, tf);

        crate::sbi::set_timer(crate::sbi::read_time() + 10_000);
    }
}
unsafe impl Send for KernelState {}
unsafe impl Sync for KernelState {}

#[cfg(test)]
mod test;
