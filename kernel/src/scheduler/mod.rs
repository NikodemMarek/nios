use alloc::vec::Vec;

use crate::{
    memory_manager::{PageTable, Pmm, Vmm, create_page_table},
    traps::TrapFrame,
};

pub struct Scheduler {
    pmm: Pmm,
    current_program: Option<usize>,
    programs: Vec<TrapFrame>,
}
impl Scheduler {
    pub fn new(pmm: Pmm) -> Self {
        Self {
            pmm,
            current_program: None,
            programs: Default::default(),
        }
    }

    pub fn add(&mut self, program_loc: usize) {
        let process_root_page_table_addr = self.pmm.alloc().expect("PMM out of pages!").into();
        let mut process_root_page_table = PageTable::new_root(process_root_page_table_addr);
        create_page_table(&mut self.pmm, &mut process_root_page_table);

        let mut process_vmm = Vmm::new(self.pmm, process_root_page_table);

        process_vmm.alloc().expect("MM out of pages");
        let stack_page_addr = process_vmm.alloc().expect("MM out of pages");
        let stack_page_addr = stack_page_addr.add(crate::memory_manager::PAGE_SIZE);

        self.programs.push(TrapFrame {
            sp: stack_page_addr.0 as u64,
            sepc: program_loc as u64,
            sstatus: 0b100100000, // SPP = 1 (Supervisor), SPIE = 1 (Enable interrupts on sret)
            satp: process_root_page_table.satp(),
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
        if self.programs.is_empty() {
            return None;
        }

        self.current_program = Some(self.current_program.map_or(0, |current_program| {
            (current_program + 1) % self.programs.len()
        }));
        self.current_program
    }
}
