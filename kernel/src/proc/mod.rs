use core::{convert::Infallible, mem, sync::atomic::AtomicU32};

use sched::Scheduler;
use x86_64::structures::paging::PageTable;

use crate::{
    context::{ContextValue, InterruptContext, InterruptContextValue},
    declare_module,
    memory::stack::Stack,
    util::OnceMutex,
};

pub mod sched;

pub type ThreadID = u32;
// TODO: Should this Thread type be turned into a Process type, or should a Process type contain a Thread?
// Separating the two might make some things easier, but could also introduce a gap between the two which could be problematic.
// I think having a Process type that contains a `Vec<Thread>` would be the best option. Just would have to ensure process switching is handled correctly.

/// A thread is a unit of execution within a process.
#[derive(Debug)]
pub struct Thread {
    pub pid: ThreadID,
    pub name: &'static str,
    pub state: ThreadState,
    pub stack: Stack,
    pub context: InterruptContextValue,
    // TODO: pub ring: PrivilegeLevel
    // TODO: pub parent: ProcessID
}

impl Thread {
    /// Creates a new thread with the given `pid`, `name`, `stack`, and `context`.
    pub fn new(
        pid: ThreadID,
        name: &'static str,
        stack: Stack,
        context: InterruptContextValue,
    ) -> Self {
        Thread {
            pid,
            name,
            state: ThreadState::Waiting,
            stack,
            context,
        }
    }
    /// Creates a new thread with the given `stack` and `context`.
    pub fn from_stack_context(stack: Stack, context: InterruptContextValue) -> Self {
        let pid = NEXT_PID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        Thread::new(pid, "main", stack, context)
    }
    /// Updates the thread's context and returns the old context.
    pub fn update_context(&mut self, context: InterruptContextValue) -> InterruptContextValue {
        mem::replace(&mut self.context, context)
    }
}

pub static NEXT_PID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Running,
    Waiting,
    Killed,
}

declare_module!("proc", init_proc);

pub static SCHEDULER: OnceMutex<Scheduler> = OnceMutex::uninitialized();

fn init_proc() -> Result<(), Infallible> {
    let scheduler = Scheduler::new();
    SCHEDULER.init(scheduler);
    Ok(())
}
// TODO: I don't think this raw pointer is really needed, as it could just be a reference.
extern "C" fn timer_handler(ctx: InterruptContext) {
    // The interrupt wrapper is guaranteed to disable interrupts and reenable them.
    let mut sch = SCHEDULER.get();
    let mut next_index = 0;
    if let Some(tid) = sch.current {
        let thread = sch
            .threads
            .get_mut(&tid)
            .expect("Current thread does not exist!");
        thread.context = *&*ctx;
        thread.state = ThreadState::Waiting
    }
}
