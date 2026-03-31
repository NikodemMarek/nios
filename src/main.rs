#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod global_allocator;
mod heap;
mod pmm;
mod shell;
mod traps;
mod uart;

use core::arch::global_asm;
use core::panic::PanicInfo;

use crate::global_allocator::GlobalAllocator;
use crate::heap::Heap;

global_asm!(include_str!("main.s"));

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::empty();

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    if cfg!(test) {
        #[cfg(test)]
        test_main();

        exit_qemu(ExitCode::Success);
    } else {
        let pmm = pmm::Pmm::new();
        let heap = Heap::new(pmm);

        ALLOCATOR.init(heap);

        shell::run();
    }

    loop {}
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
    println!("{}", info.message());
    loop {}
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}
