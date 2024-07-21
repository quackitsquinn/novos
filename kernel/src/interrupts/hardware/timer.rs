use x86_64::structures::idt::InterruptStackFrame;

use crate::println;

use super::InterruptIndex;

static mut TICKS: u64 = 0;

pub const TIMER_FREQUENCY: f32 = 18.2065; // stolen from https://wiki.osdev.org/Programmable_Interval_Timer

pub(super) extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        TICKS += 1;

        super::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.into());
    }
}

pub fn get_ticks() -> u64 {
    unsafe { TICKS }
}

pub fn get_seconds() -> u64 {
    unsafe { TICKS / (TIMER_FREQUENCY as u64) }
}

pub fn get_minutes() -> u64 {
    get_seconds() / 60
}

pub fn get_hours() -> u64 {
    get_minutes() / 60
}
