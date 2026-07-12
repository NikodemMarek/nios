#![no_std]
#![no_main]

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::panic::PanicInfo;

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
    run(&mut Uart::read, &mut Uart);
    loop {}
}

enum Command {
    Echo(String),
    Exit,
}

pub fn run<I, O>(i: &mut I, o: &mut O)
where
    I: FnMut() -> u8,
    O: core::fmt::Write + Copy,
{
    let mut write_o = *o;
    let mut write = |buffer: &str| {
        clear_line(&mut write_o);
        write!(write_o, "> {}", buffer).unwrap();
    };

    loop {
        write!(o, "> ").unwrap();
        let input = read_line(i, &mut write);
        writeln!(o).unwrap();

        let command = parse(&input);
        match command {
            Ok(command) => match command {
                Command::Echo(message) => writeln!(o, "{message}").unwrap(),
                Command::Exit => {
                    writeln!(o, "exiting shell").unwrap();
                    break;
                }
            },
            Err(err) => {
                writeln!(o, "{err}").unwrap();
            }
        }
    }
}

fn parse(input: &str) -> Result<Command, String> {
    let mut parts = input.split(' ');
    match parts.next() {
        Some("echo") => {
            let message = parts.collect::<Vec<_>>().join(" ");
            Ok(Command::Echo(message))
        }
        Some("ex") => Ok(Command::Exit),
        Some(command) => Err(format!("Unknown command: {command}")),
        None => Err("No command provided".to_string()),
    }
}

fn read_line<I, O>(i: &mut I, write: &mut O) -> String
where
    O: FnMut(&str),
    I: FnMut() -> u8,
{
    let mut buffer = String::with_capacity(128);

    loop {
        let char = i();

        match char {
            13 => {
                write(&buffer);
                return buffer;
            }
            127 => {
                buffer.pop();
                write(&buffer);
            }
            _ => {
                buffer.push(char as char);
                write(&buffer);
            }
        }
    }
}

fn clear_line(o: &mut impl core::fmt::Write) {
    write!(o, "\r").unwrap();
    for _ in 0..200 {
        write!(o, " ").unwrap();
    }
    write!(o, "\r").unwrap();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
