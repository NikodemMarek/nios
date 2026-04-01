#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod global_allocator;
mod heap;
mod memory_manager;
mod shell;
mod traps;
mod uart;

use core::arch::global_asm;
use core::panic::PanicInfo;

use crate::global_allocator::GlobalAllocator;
use crate::heap::Heap;
use crate::memory_manager::Pmm;

global_asm!(include_str!("main.s"));

#[global_allocator]
static ALLOCATOR: GlobalAllocator<Pmm> = GlobalAllocator::empty();

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    if cfg!(test) {
        #[cfg(test)]
        test_main();

        exit_qemu(ExitCode::Success);
    } else {
        let pmm = Pmm::init();
        let heap = Heap::new(pmm);

        ALLOCATOR.init(heap);

        shell::run();

        loop {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ExitCode {
    Success = 0x5555,
    Fail = 0x3333,
}
pub fn exit_qemu(code: ExitCode) -> ! {
    use core::ptr::write_volatile;

    unsafe {
        write_volatile(0x100000 as *mut u32, code as u32);
    }
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if cfg!(test) {
        println!("\x1b[31mFAILED\x1b[0m");
        println!("Error: {}\n", info);

        exit_qemu(ExitCode::Fail);
    } else {
        println!("Kernel panicked: {}", info.message());

        loop {}
    }
}

#[cfg(test)]
pub trait Testable {
    fn run(&self);
}

#[cfg(test)]
impl<T: Fn()> Testable for T {
    fn run(&self) {
        print!("test {} ... ", core::any::type_name::<T>());
        self();
        println!("\x1b[32mOK\x1b[0m");
    }
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
}
