//! Timer interrupt handler and utilities.


use crate::{context::InterruptContext, interrupt_wrapper, proc::sched_next};

use super::InterruptIndex;

/// The frequency of the timer interrupt.
pub const TIMER_FREQUENCY: f32 = 18.2065; // stolen from https://wiki.osdev.org/Programmable_Interval_Timer

pub(super) extern "C" fn timer_handler(frame: InterruptContext) {
    unsafe {
        sched_next(frame);

        super::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.into());
    }
}

interrupt_wrapper!(timer_handler, timer_handler_raw);
