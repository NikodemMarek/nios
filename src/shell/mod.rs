use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Write;

use crate::uart::Uart;

enum Command {
    Echo(String),
}

pub fn run() {
    fn write(buffer: &str) {
        clear_line();
        write!(Uart, "> {}", buffer).unwrap();
    }

    loop {
        write!(Uart, "> ").unwrap();
        let input = read_line(write);
        writeln!(Uart).unwrap();

        let command = parse(&input);
        match command {
            Ok(command) => match command {
                Command::Echo(message) => writeln!(Uart, "{message}").unwrap(),
            },
            Err(err) => {
                writeln!(Uart, "{err}").unwrap();
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
        Some(command) => Err(format!("Unknown command: {command}")),
        None => Err("No command provided".to_string()),
    }
}

fn read_line(write: fn(&str)) -> String {
    let mut buffer = String::with_capacity(128);

    loop {
        let char = Uart::read();

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

fn clear_line() {
    let _ = write!(Uart, "\r");
    for _ in 0..250 {
        let _ = write!(Uart, " ");
    }
    let _ = write!(Uart, "\r");
}
