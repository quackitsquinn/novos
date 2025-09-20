use cake::{limine::mp::Cpu, spin::Once};

pub struct CoreContext {
    pub(super) stack_start: Once<u64>,
}

impl CoreContext {
    pub(super) const fn new(cpu: &Cpu) -> Self {
        Self {
            stack_start: Once::new(),
        }
    }

    pub fn get_stack_start(&self) -> u64 {
        *self.stack_start.wait()
    }
}
