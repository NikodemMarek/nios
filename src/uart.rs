use core::{fmt::Write, ptr::copy_nonoverlapping};

use crate::heap::Heap;

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

pub fn read_line(heap: &mut Heap) -> &mut [u8] {
    let mut size = 64;
    let mut buffer = heap.alloc_array(size);

    let mut i = 0;
    loop {
        let char = Uart::read();
        let _ = write!(Uart, "{}", char as char);

        match char {
            13 => return buffer,
            127 => {
                i -= 1;
                buffer[i] = 0;
                clear_line();
                let _ = write!(Uart, "{}", core::str::from_utf8(buffer).unwrap());
            }
            _ => {
                buffer[i] = char;
                i += 1;
            }
        }

        // Resize the buffer to fit the input
        if i == size {
            let new_size = if size > 512 { size + 512 } else { size * 2 };
            let new_buffer = heap.alloc_array(new_size);
            unsafe {
                copy_nonoverlapping(buffer.as_ptr(), new_buffer.as_mut_ptr(), size);
            }

            size = new_size;
            heap.free(buffer.as_mut_ptr());
            buffer = new_buffer;
        }
    }
}

pub fn clear_line() {
    let _ = write!(Uart, "\r");
    for _ in 0..250 {
        let _ = write!(Uart, " ");
    }
    let _ = write!(Uart, "\r");
}
