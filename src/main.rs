#![no_std]
#![no_main]

mod heap;
mod pmm;
mod uart;

use core::arch::global_asm;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::ptr::copy_nonoverlapping;

use crate::uart::Uart;

global_asm!(include_str!("main.s"));

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let pmm = pmm::Pmm::new();
    let mut heap = heap::Heap::new(pmm);

    let block_ptr = heap.malloc(64);
    unsafe {
        let msg = b"Hello malloc!\0";
        copy_nonoverlapping(msg.as_ptr(), block_ptr as *mut u8, msg.len());

        let cstr = core::ffi::CStr::from_ptr(block_ptr as *const core::ffi::c_char);
        if let Ok(s) = cstr.to_str() {
            let _ = writeln!(Uart, "{}", s);
        }
    };

    let buffer = heap.alloc_array(100);
    loop {
        uart::read_line(buffer);
        let _ = writeln!(Uart, "entered: {}", core::str::from_utf8(buffer).unwrap());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(machine_cause: u32) {
    let is_exception = (machine_cause >> 31) & 1 == 0;
    let cause = machine_cause & 0b01111111111111111111111111111111;

    if is_exception {
        let cause_str = match cause {
            0 => "Instruction Address Misaligned",
            1 => "Instruction Access Fault",
            2 => "Illegal Instruction",
            3 => "Breakpoint",
            4 => "Load Address Misaligned",
            8 => "Environment Call (U-mode)",
            11 => "Environment Call (M-mode)",
            12 => "Instruction Page Fault",
            15 => "Store Page Fault",
            _ => "Unknown",
        };
        let _ = write!(Uart, "Exception trap called, cause: [{cause}] {cause_str}");
        todo!("handle exception")
    } else {
        let cause_str = match cause {
            3 => "Machine Software Interrupt",
            7 => "Machine Timer Interrupt",
            11 => "Machine External Interrupt",
            _ => "Unknown",
        };
        let _ = write!(Uart, "Interrupt trap called, cause: [{cause}] {cause_str}");
        todo!("handle interrupt")
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
