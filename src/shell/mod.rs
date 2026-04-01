use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{print, println, uart::Uart};

enum Command {
    Echo(String),
    Exit,
}

pub fn run() {
    fn write(buffer: &str) {
        clear_line();
        print!("> {}", buffer);
    }

    loop {
        print!("> ");
        let input = read_line(write);
        println!();

        let command = parse(&input);
        match command {
            Ok(command) => match command {
                Command::Echo(message) => println!("{message}"),
                Command::Exit => {
                    println!("exiting shell");
                    break;
                }
            },
            Err(err) => {
                println!("{err}");
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
    print!("\r");
    for _ in 0..200 {
        print!(" ");
    }
    print!("\r");
}
