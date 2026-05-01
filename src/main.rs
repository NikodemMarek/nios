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
mod scheduler;
mod shell;
mod traps;
mod uart;

use core::cell::{RefCell, RefMut};

use crate::global_allocator::GlobalAllocator;
use crate::heap::Heap;
use crate::memory_manager::Vmm;
use crate::scheduler::Scheduler;

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
        let mut vmm = Vmm::new(pmm, root_page_table);
        let heap = Heap::new(vmm);

        ALLOCATOR.init(heap);
        STATE.init();

        STATE
            .get()
            .add(&mut vmm, dummy_program as *const () as usize);
        STATE.get().add(&mut vmm, shell as *const () as usize);

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
    pub fn init(&self) {
        *self.0.borrow_mut() = Some(Scheduler::new());
    }

    #[inline]
    fn get(&self) -> RefMut<'_, Scheduler> {
        RefMut::map(self.0.borrow_mut(), |mi| {
            mi.as_mut().expect("Kernel state not initialized")
        })
    }

    fn switch_task(&self, tf: &mut traps::TrapFrame) {
        let mut scheduler = self.get();

        let process_number = scheduler.current_program;
        scheduler.save(process_number, tf);

        let next_process = scheduler.next().expect("No process left to execute!");
        scheduler.restore(next_process, tf);

        crate::sbi::set_timer(crate::sbi::read_time() + 1_000_000);
    }
}
unsafe impl Send for KernelState {}
unsafe impl Sync for KernelState {}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text")]
pub extern "C" fn shell() {
    shell::run(&mut crate::uart::Uart::read, &mut crate::uart::Uart);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text")]
pub extern "C" fn dummy_program() {
    use core::fmt::Write;
    writeln!(
        crate::sbi::Sbi,
        "hey I'm a dummy program! I'll scream until you stop me"
    )
    .unwrap();
    loop {}
}

#[cfg(test)]
mod test;
