#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ExitCode {
    Success = 0x5555,
    Fail = 0x3333,
}

pub fn exit(code: ExitCode) -> ! {
    use core::ptr::write_volatile;

    unsafe {
        write_volatile(0xffffffff00100000 as *mut u32, code as u32);
    }
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
