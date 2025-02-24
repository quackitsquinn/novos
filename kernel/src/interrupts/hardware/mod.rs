use core::convert::Infallible;

use pic8259::ChainedPics;
use spin::Mutex;

use crate::declare_module;

pub mod timer;

// TODO: APIC check

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl Into<u8> for InterruptIndex {
    fn into(self) -> u8 {
        self as u8
    }
}

impl Into<usize> for InterruptIndex {
    fn into(self) -> usize {
        self as usize
    }
}

pub(super) fn define_hardware() {
    super::IDT
        .modify()
        .set_handler(InterruptIndex::Timer as u8, timer::timer_handler);
}

declare_module!("hardware_interrupts", init);

fn init() -> Result<(), Infallible> {
    unsafe {
        let mut p = PICS.lock();
        // Unmask interrupts (afaik it's lsb first? idk)
        //p.write_masks(0b11111110, 0b11111111);
        p.initialize();
    }
    x86_64::instructions::interrupts::enable();
    Ok(())
}
