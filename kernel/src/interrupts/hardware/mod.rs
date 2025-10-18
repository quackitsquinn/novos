//! Rust abstractions for working with hardware interrupts
use core::{convert::Infallible, mem::transmute};

use cake::Mutex;
use pic8259::ChainedPics;

use crate::declare_module;

// TODO: This module should be kept for legacy PIC8259 support, but should only be initialized if no APIC is present.

pub mod timer;

/// The IRQ offset for the primary PIC.
pub const PIC_1_OFFSET: u8 = 32;
/// The IRQ offset for the secondary PIC.
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// The chained PICs.
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
/// Interrupt indices for hardware interrupts.
pub enum InterruptIndex {
    /// Timer interrupt.
    Timer = PIC_1_OFFSET,
    /// Keyboard interrupt.
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
    let mut idt = super::IDT.modify();
    idt[InterruptIndex::Timer as u8]
        .set_handler_fn(unsafe { transmute(timer::timer_handler_raw as *mut ()) });
    drop(idt);
    unsafe {
        super::IDT.commit();
    }
}

declare_module!("hardware_interrupts", init);

fn init() -> Result<(), Infallible> {
    unsafe {
        let mut p = PICS.lock();
        // Unmask interrupts (afaik it's lsb first? idk)
        p.write_masks(0b11111110, 0b11111111);
        p.initialize();
    }
    Ok(())
}

/// Fully disables hardware interrupts.
pub unsafe fn disable() {
    let mut pics = PICS.lock();
    unsafe {
        pics.disable();
    }
}
