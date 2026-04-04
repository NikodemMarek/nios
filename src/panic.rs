use core::panic::PanicInfo;

use crate::{println, qemu};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if cfg!(test) {
        println!("\x1b[31mFAILED\x1b[0m");
        println!("Error: {}\n", info);

        qemu::exit(qemu::ExitCode::Fail);
    } else {
        println!("Kernel panicked: {}", info.message());
        println!("             at: {}", info.location().unwrap());

        loop {}
    }
}
