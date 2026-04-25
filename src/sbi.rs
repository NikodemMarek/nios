#[inline(always)]
fn sbi_call(
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    fid: usize,
    eid: usize,
) -> (isize, isize) {
    let (error, value);
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") eid,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    (error, value)
}

fn print(s: &str) {
    let ptr = s.as_bytes().as_ptr() as usize;
    sbi_call(s.len(), ptr, 0, 0, 0, 0, 0, 0x4442434E);
}
pub struct Sbi;
impl core::fmt::Write for Sbi {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print(s);
        Ok(())
    }
}
