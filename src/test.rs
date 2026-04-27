use core::fmt::Write;

use crate::uart::Uart;

#[cfg(test)]
pub trait Testable {
    fn run(&self);
}

#[cfg(test)]
impl<T: Fn()> Testable for T {
    fn run(&self) {
        write!(Uart, "test {} ... ", core::any::type_name::<T>());
        self();
        writeln!(Uart, "\x1b[32mOK\x1b[0m");
    }
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    writeln!(Uart, "Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
}
