#![no_std]
#![no_main]

mod heap;
mod pmm;
mod shell;
mod uart;

use core::alloc::GlobalAlloc;
use core::arch::global_asm;
use core::cell::{RefCell, RefMut};
use core::fmt::Write;
use core::panic::PanicInfo;

use crate::heap::Heap;
use crate::uart::Uart;

global_asm!(include_str!("main.s"));

#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator::empty();

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let pmm = pmm::Pmm::new();
    let heap = Heap::new(pmm);

    {
        let mut allocator = ALLOCATOR.0.borrow_mut();
        *allocator = Some(heap);
    }

    loop {}
}

pub struct GlobalAllocator(RefCell<Option<Heap>>);
impl GlobalAllocator {
    #[inline]
    const fn empty() -> Self {
        GlobalAllocator(RefCell::new(None))
    }

    #[inline]
    fn get(&self) -> core::cell::RefMut<'_, Heap> {
        RefMut::map(self.0.borrow_mut(), |mi| {
            mi.as_mut().expect("Allocator not initialized")
        })
    }
}
unsafe impl GlobalAlloc for GlobalAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.get().malloc(layout.size() as u64) as *mut u8
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.get().free(ptr);
    }
}
unsafe impl Send for GlobalAllocator {}
unsafe impl Sync for GlobalAllocator {}

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(machine_cause: u32) {
    let is_exception = (machine_cause >> 31) & 1 == 0;
    let cause = machine_cause & 0b01111111111111111111111111111111;

    if is_exception {
        let cause_str = match cause {
            0 => "Instruction Address Misaligned",
            1 => "Instruction Access Fault",
            2 => "Illegal Instruction",
            3 => "Breakpoint",
            4 => "Load Address Misaligned",
            8 => "Environment Call (U-mode)",
            11 => "Environment Call (M-mode)",
            12 => "Instruction Page Fault",
            15 => "Store Page Fault",
            _ => "Unknown",
        };
        let _ = write!(Uart, "Exception trap called, cause: [{cause}] {cause_str}");
        todo!("handle exception")
    } else {
        let cause_str = match cause {
            3 => "Machine Software Interrupt",
            7 => "Machine Timer Interrupt",
            11 => "Machine External Interrupt",
            _ => "Unknown",
        };
        let _ = write!(Uart, "Interrupt trap called, cause: [{cause}] {cause_str}");
        todo!("handle interrupt")
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
