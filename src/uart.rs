use core::fmt::Write;

pub struct Uart;
impl Uart {
    const ADDRESS: *mut u8 = 0x10000000 as *mut u8;

    fn print(s: &str) {
        for c in s.bytes() {
            unsafe {
                Uart::ADDRESS.write_volatile(c);
            }
        }
    }
}
impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Uart::print(s);
        Ok(())
    }
}
