#![no_std]
#![no_main]

mod pmm;
mod uart;

use core::arch::global_asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use crate::uart::Uart;

global_asm!(include_str!("main.s"));

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let mut pmm = pmm::Pmm::new();

    let p1 = pmm.alloc();
    let p2 = pmm.alloc();
    pmm.free(p1.index);
    pmm.free(p2.index);

    let _ = writeln!(Uart, "Hello from nios!");

    loop {}
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
