use core::{fmt::Write, ptr::copy_nonoverlapping};

use crate::{heap::Heap, uart::Uart};

pub fn run(heap: &mut Heap) {
    fn write(buffer: &mut [u8]) {
        clear_line();
        write!(Uart, "> {}", core::str::from_utf8(buffer).unwrap()).unwrap();
    }

    loop {
        write!(Uart, "> ").unwrap();
        let input = read_line(heap, write);
        writeln!(Uart).unwrap();

        writeln!(Uart, "prompt: {}", core::str::from_utf8(input).unwrap()).unwrap();
    }
}

pub fn read_line(heap: &mut Heap, write: fn(&mut [u8])) -> &mut [u8] {
    let mut size = 64;
    let mut buffer = heap.alloc_array(size);

    let mut i = 0;
    loop {
        let char = Uart::read();

        match char {
            13 => {
                write(buffer);
                return buffer;
            }
            127 => {
                i -= 1;
                buffer[i] = 0;
                write(buffer);
            }
            _ => {
                buffer[i] = char;
                i += 1;
                write(buffer);
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
