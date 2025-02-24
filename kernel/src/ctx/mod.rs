use core::arch::asm;

use log::info;
use x86_64::structures::idt::InterruptStackFrame;

mod contexts;
mod int_context;

pub use contexts::Context;
pub use contexts::PageFaultInterruptContext;
pub use int_context::InterruptContext;

#[allow(private_bounds)] // Don't let implementations on arbitrary types
pub trait ProcessorContext: Sealed {}

trait Sealed {}

impl Sealed for Context {}
impl Sealed for PageFaultInterruptContext {}
impl Sealed for InterruptContext {}

impl ProcessorContext for Context {}
impl ProcessorContext for PageFaultInterruptContext {}
impl ProcessorContext for InterruptContext {}
