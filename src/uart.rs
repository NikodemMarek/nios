#[derive(Copy, Clone)]
pub struct Uart;
impl Uart {
    pub const OFFSET: usize = 0x10000000;

    // This is a test workaround, ideally UART should know it it is in the higher-half or not.
    // #[cfg(test)]
    // const ADDRESS: *mut u8 = Uart::OFFSET as *mut u8;
    // #[cfg(not(test))]
    const ADDRESS: *mut u8 = (0xffffffff00000000 + Uart::OFFSET) as *mut u8;

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
