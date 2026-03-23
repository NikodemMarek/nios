#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(
    ".section .text.boot",
    ".global _start",
    "_start:",
    "    la sp, 0x88000000",
    "    call kernel_main",
    "1:  wfi",
    "    j 1b",
);

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
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
