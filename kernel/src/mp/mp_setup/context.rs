use alloc::vec::Vec;
use cake::{
    limine::mp::Cpu,
    spin::{Once, RwLock},
};

pub struct CoreContext {
    pub(super) stack_start: Once<u64>,
    pub(super) tasks: Vec<fn() -> ()>,
}

impl CoreContext {
    pub(super) fn new(cpu: &Cpu) -> Self {
        Self {
            stack_start: Once::new(),
            tasks: Vec::with_capacity(5),
        }
    }

    pub fn get_stack_start(&self) -> &Once<u64> {
        &self.stack_start
    }

    pub fn add_task(&mut self, task: fn() -> ()) {
        self.tasks.push(task);
    }
}
