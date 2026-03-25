#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("trap.s"));

const UART: *mut u8 = 0x10000000 as *mut u8;

fn uart_print(s: &str) {
    for c in s.bytes() {
        unsafe {
            UART.write_volatile(c);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    uart_print("Hello from nios!\n");
    unsafe { core::arch::asm!("ecall") };
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(machine_cause: u64) -> () {
    if machine_cause == 8 {
        uart_print("ecall trap called!\n");
        todo!("handle ecall")
    } else {
        uart_print("error trap called!\n");
        todo!("handle error")
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
