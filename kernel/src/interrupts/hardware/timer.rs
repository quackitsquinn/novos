use core::{hint::spin_loop, time::Duration};

use x86_64::{instructions::interrupts::without_interrupts, structures::idt::InterruptStackFrame};

use crate::{context::InterruptContext, interrupt_wrapper, println, proc::sched_next};

use super::InterruptIndex;

// TODO: Refactor into an atomic type
static mut TICKS: u64 = 0;

pub const TIMER_FREQUENCY: f32 = 18.2065; // stolen from https://wiki.osdev.org/Programmable_Interval_Timer

pub(super) extern "C" fn timer_handler(frame: InterruptContext) {
    unsafe {
        TICKS += 1;

        sched_next(frame);

        super::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.into());
    }
}

interrupt_wrapper!(timer_handler, timer_handler_raw);

pub fn get_ticks() -> u64 {
    without_interrupts(|| unsafe { TICKS })
}

pub fn get_seconds() -> u64 {
    get_ticks() / (TIMER_FREQUENCY as u64)
}

pub fn get_minutes() -> u64 {
    get_seconds() / 60
}

pub fn get_hours() -> u64 {
    get_minutes() / 60
}

pub fn sleep(time: Duration) {
    let timer = Timer::new(time);
    while !timer.is_done() {
        spin_loop();
    }
}

pub struct Timer {
    pub ticks: u64,
    pub end: u64,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        let ticks = (duration.as_secs_f32() * TIMER_FREQUENCY) as u64;
        Self {
            ticks,
            end: get_ticks() + ticks,
        }
    }

    pub fn is_done(&self) -> bool {
        get_ticks() >= self.end
    }
}
