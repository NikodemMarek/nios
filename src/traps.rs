use crate::println;

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(cause: u64) {
    let is_exception = (cause >> 63) & 1 == 0;
    let cause_code = cause & 0x7fffffffffffffff;

    if is_exception {
        let cause_str = match cause_code {
            0 => "Instruction Address Misaligned",
            1 => "Instruction Access Fault",
            2 => "Illegal Instruction",
            3 => "Breakpoint",
            4 => "Load Address Misaligned",
            5 => "Load Access Fault",
            6 => "Store Address Misaligned",
            7 => "Store Access Fault",
            8 => "Environment Call (U-mode)",
            9 => "Environment Call (S-mode)",
            11 => "Environment Call (M-mode)",
            12 => "Instruction Page Fault",
            13 => "Load Page Fault",
            15 => "Store Page Fault",
            _ => "Unknown",
        };
        println!("Exception trap called, cause: [{cause_code}] {cause_str}");
        todo!("handle exception")
    } else {
        let cause_str = match cause_code {
            1 => "Supervisor Software Interrupt",
            3 => "Machine Software Interrupt",
            5 => "Supervisor Timer Interrupt",
            7 => "Machine Timer Interrupt",
            9 => "Supervisor External Interrupt",
            11 => "Machine External Interrupt",
            _ => "Unknown",
        };
        println!("Interrupt trap called, cause: [{cause_code}] {cause_str}");
        todo!("handle interrupt")
    }
}
