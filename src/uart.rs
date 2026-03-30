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
impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Uart::print(s);
        Ok(())
    }
}

pub fn read_line(buffer: &mut [u8]) {
    let mut i = 0;
    loop {
        let char = Uart::read();
        let _ = write!(Uart, "{}", char as char);

        match char {
            13 => break,
            127 => {
                i -= 1;
                buffer[i] = 0;
                for _ in 0..100 {
                    let _ = write!(Uart, " ");
                }
                let _ = write!(Uart, "\r{}", core::str::from_utf8(buffer).unwrap());
            }
            _ => {
                buffer[i] = char;
                i += 1;
            }
        }
    }
}
