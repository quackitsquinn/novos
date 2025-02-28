use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use x86_64::{structures::gdt::SegmentSelector, VirtAddr};

use crate::{
    context::{InterruptContext, InterruptContextValue},
    gdt::GDT,
    memory::stack::{Stack, StackFlags},
};

use super::{Thread, ThreadID, ThreadState};

pub struct Scheduler {
    // Would using a VecDeque or LinkedList be better?
    // Threads that terminate are removed from the list, so it might be ideal?
    pub threads: BTreeMap<ThreadID, Thread>,
    pub current: Option<ThreadID>,
    pub index: usize,
}
// TODO: Can `extern "C"` be safely removed?
pub type ThreadEntry = extern "C" fn() -> !;

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            threads: BTreeMap::new(),
            current: None,
            index: 0,
        }
    }

    pub fn add_thread(&mut self, thread: Thread) {
        self.threads.insert(thread.pid, thread);
    }

    pub fn remove_thread(&mut self, pid: ThreadID) {
        self.threads.remove(&pid);
    }

    pub fn spawn(&mut self, entry: ThreadEntry) {
        let stack = Stack::allocate_kernel_stack(0x4000, StackFlags::RWKernel)
            .expect("Failed to allocate stack");
        let context = unsafe {
            InterruptContextValue::new(
                VirtAddr::new(entry as u64),
                stack.get_stack_base(),
                GDT.1.code_selector(),
            )
        };
        // TODO: Probably should be an Arc<Thread> or something similar. Really want to avoid Arc<Mutex<Thread>> though.
        let thread = Thread::from_stack_context(stack, context);
        self.add_thread(thread);
    }
    pub fn handle_timer(&mut self, ctx: InterruptContext) {
        if self.threads.is_empty() {
            // No threads to schedule, just return and continue execution.
            return;
        }
        if let Some(tid) = self.current {
            let thread = self
                .threads
                .get_mut(&tid)
                .expect("Current thread does not exist!");
            thread.context = *&*ctx;
            thread.state = ThreadState::Waiting
        }
        let key = *self
            .threads
            .keys()
            .cycle()
            .nth(self.index)
            .expect("Infallible");
        self.index += 1;
        let thread = self.threads.get_mut(&key).expect("Thread does not exist!");
        thread.state = ThreadState::Running;
        self.current = Some(key);
        unsafe {
            ctx.modify().switch(&mut thread.context);
        }
    }
}
