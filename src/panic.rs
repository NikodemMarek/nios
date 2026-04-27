use core::{fmt::Write, panic::PanicInfo};

use crate::{qemu, uart::Uart};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if cfg!(test) {
        writeln!(Uart, "\x1b[31mFAILED\x1b[0m").unwrap();
        writeln!(Uart, "Error: {}\n", info).unwrap();

        qemu::exit(qemu::ExitCode::Fail);
    } else {
        writeln!(Uart, "Kernel panicked: {}", info.message()).unwrap();
        writeln!(Uart, "             at: {}", info.location().unwrap()).unwrap();

        loop {}
    }
}
