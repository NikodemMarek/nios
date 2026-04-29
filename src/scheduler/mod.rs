const MAX_PROGRAMS_COUNT: usize = 20;

pub struct Scheduler {
    current_program: usize,
    programs_count: usize,
    programs: [usize; MAX_PROGRAMS_COUNT],
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            current_program: 0,
            programs_count: 0,
            programs: Default::default(),
        }
    }

    pub fn add(&mut self, program_loc: usize) {
        assert!(
            self.programs_count < MAX_PROGRAMS_COUNT,
            "Implementation constraint surpassed, too many programs loaded to scheduler!"
        );

        self.programs[self.programs_count] = program_loc;
        self.programs_count += 1;
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

        Some(self.programs[self.current_program])
    }
}
