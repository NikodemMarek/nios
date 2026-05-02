use core::fmt::Write;

core::arch::global_asm!(include_str!("traps.s"));

#[repr(C)]
#[derive(Clone, Default)]
pub struct TrapFrame {
    pub ra: u64,
    pub sp: u64,
    pub gp: u64,
    pub tp: u64,
    pub t0_t2: [u64; 3],
    pub fp: u64,
    pub s1: u64,
    pub a0_a7: [u64; 8],
    pub s2_s11: [u64; 10],
    pub t3_t6: [u64; 4],
    pub sepc: u64,
    pub sstatus: u64,
    pub _padding: u64,
}
impl core::fmt::Display for TrapFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut write_reg = |l: &str, reg: u64| -> Result<_, _> {
            writeln!(f, "{l} {reg:064b} | {reg:#018x} | {reg}",)
        };

        write_reg("    ra:", self.ra)?;
        write_reg("    sp:", self.sp)?;
        write_reg("    gp:", self.gp)?;
        write_reg("    tp:", self.tp)?;
        write_reg(" t0-t2:", self.t0_t2[0])?;
        write_reg("       ", self.t0_t2[1])?;
        write_reg("       ", self.t0_t2[2])?;
        write_reg("    fp:", self.fp)?;
        write_reg("    s1:", self.s1)?;
        write_reg(" a0-a7:", self.a0_a7[0])?;
        write_reg("       ", self.a0_a7[1])?;
        write_reg("       ", self.a0_a7[2])?;
        write_reg("       ", self.a0_a7[3])?;
        write_reg("       ", self.a0_a7[4])?;
        write_reg("       ", self.a0_a7[5])?;
        write_reg("       ", self.a0_a7[6])?;
        write_reg("       ", self.a0_a7[7])?;
        write_reg("s2-s11:", self.s2_s11[0])?;
        write_reg("       ", self.s2_s11[1])?;
        write_reg("       ", self.s2_s11[2])?;
        write_reg("       ", self.s2_s11[3])?;
        write_reg("       ", self.s2_s11[4])?;
        write_reg("       ", self.s2_s11[5])?;
        write_reg("       ", self.s2_s11[6])?;
        write_reg("       ", self.s2_s11[7])?;
        write_reg("       ", self.s2_s11[8])?;
        write_reg("       ", self.s2_s11[9])?;
        write_reg(" t3-t6:", self.t3_t6[0])?;
        write_reg("       ", self.t3_t6[1])?;
        write_reg("       ", self.t3_t6[2])?;
        write_reg("       ", self.t3_t6[3])?;
        write_reg("  sepc:", self.sepc)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(tf: &mut TrapFrame, scause: u64, stval: u64) {
    let is_exception = (scause >> 63) & 1 == 0;
    let cause_code = scause & 0x7fffffffffffffff;

    if is_exception {
        let cause_str = match cause_code {
            0 => "Instruction Address Misaligned",
            1 => "Instruction Access Fault",
            2 => {
                panic!("Illegal Instruction - {stval:#064b}")
            }
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
            15 => {
                panic!("Unhandled Store Page Fault - tried to write to {stval:#x}");
            }
            _ => "Unknown",
        };
        panic!(
            "Unhandled exception trap, cause: [{cause_code}] {cause_str} with value: {stval:#064b}"
        );
    } else {
        let cause_str = match cause_code {
            1 => "Supervisor Software Interrupt",
            3 => "Machine Software Interrupt",
            5 => {
                crate::sbi::reset_timer();

                crate::STATE.switch_task(tf);

                return;
            }
            7 => "Machine Timer Interrupt",
            9 => "Supervisor External Interrupt",
            11 => "Machine External Interrupt",
            _ => "Unknown",
        };

        panic!("Unhandled interrupt trap, cause: [{cause_code}] {cause_str}");
    }
}
