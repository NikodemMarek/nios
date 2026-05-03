use crate::{
    memory_manager::{MemoryManager, Vmm},
    traps::TrapFrame,
};

const MAX_PROGRAMS_COUNT: usize = 20;

pub struct Scheduler {
    pub current_program: usize,
    programs_count: usize,
    programs: [TrapFrame; MAX_PROGRAMS_COUNT],
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            // Kernel jumps to the trap for the first time, with its own state, so we just discard
            // it's state to some high location.
            current_program: 19,
            programs_count: 0,
            programs: Default::default(),
        }
    }

    pub fn add(&mut self, mm: &mut Vmm, program_loc: usize) {
        assert!(
            self.programs_count < MAX_PROGRAMS_COUNT,
            "Implementation constraint surpassed, too many programs loaded to scheduler!"
        );

        let stack_page_ptr = mm.alloc().expect("MM out of pages");
        let stack_page_ptr =
            unsafe { (stack_page_ptr as *const u8).add(crate::memory_manager::PAGE_SIZE) };

        self.programs[self.programs_count] = TrapFrame {
            sp: stack_page_ptr as u64,
            sepc: program_loc as u64,
            sstatus: 0b100100000, // SPP = 1 (Supervisor), SPIE = 1 (Enable interrupts on sret)
            ..Default::default()
        };
        self.programs_count += 1;
    }

    pub fn save(&mut self, process_number: usize, tf: &TrapFrame) {
        self.programs[process_number] = tf.clone();
    }
    pub fn restore(&mut self, process_number: usize, tf: &mut TrapFrame) {
        *tf = self.programs[process_number].clone();
    }
}
impl Iterator for Scheduler {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.programs_count == 0 {
            return None;
        }

        self.current_program = if self.current_program < self.programs_count - 1 {
            self.current_program + 1
        } else {
            0
        };

        Some(self.current_program)
    }
}
