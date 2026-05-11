use alloc::vec::Vec;

use crate::{
    memory_manager::{MemoryManager, Vmm},
    traps::TrapFrame,
};

pub struct Scheduler {
    current_program: Option<usize>,
    programs: Vec<TrapFrame>,
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            current_program: None,
            programs: Default::default(),
        }
    }

    pub fn add(&mut self, mm: &mut Vmm, program_loc: usize) {
        let stack_page_ptr = mm.alloc().expect("MM out of pages");
        let stack_page_ptr =
            unsafe { (stack_page_ptr as *const u8).add(crate::memory_manager::PAGE_SIZE) };

        self.programs.push(TrapFrame {
            sp: stack_page_ptr as u64,
            sepc: program_loc as u64,
            sstatus: 0b100100000, // SPP = 1 (Supervisor), SPIE = 1 (Enable interrupts on sret)
            ..Default::default()
        });
    }

    pub fn save(&mut self, tf: &TrapFrame) {
        let Some(current_program) = self.current_program else {
            return;
        };
        self.programs[current_program] = tf.clone();
    }
    pub fn restore(&mut self, process_number: usize, tf: &mut TrapFrame) {
        assert!(
            self.programs.len() > process_number,
            "Tried to restore nonexistent process!"
        );
        *tf = self.programs[process_number].clone();
    }
}
impl Iterator for Scheduler {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.programs.len() == 0 {
            return None;
        }

        self.current_program = Some(self.current_program.map_or(0, |current_program| {
            (current_program + 1) % self.programs.len()
        }));
        self.current_program
    }
}
