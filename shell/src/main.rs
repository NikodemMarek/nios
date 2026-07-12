#![no_std]
#![no_main]

use core::{fmt::Write, panic::PanicInfo};

#[derive(Copy, Clone)]
pub struct Uart;
impl Uart {
    const ADDRESS: *mut u8 = 0xffffffff10000000 as *mut u8;

    fn print(s: &str) {
        for c in s.bytes() {
            unsafe {
                Uart::ADDRESS.write_volatile(c);
            }
        }
    }

    pub fn read() -> u8 {
        let lsr_ptr = unsafe { Uart::ADDRESS.add(5) };

        loop {
            let is_byte_available = unsafe { *lsr_ptr } & 0b1 == 0b1;
            if is_byte_available {
                break;
            }
        }

        unsafe { *Uart::ADDRESS }
    }
}
impl core::fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Uart::print(s);
        Ok(())
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text")]
pub extern "C" fn _start() -> ! {
    writeln!(Uart, "Hello World!").unwrap();
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
