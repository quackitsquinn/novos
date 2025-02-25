

mod contexts;
mod int_context;

pub use contexts::Context;
pub use contexts::PageFaultInterruptContext;
pub use int_context::InterruptCodeContext;
pub use int_context::InterruptContext;

#[allow(private_bounds)] // Don't let implementations on arbitrary types
pub trait ProcessorContext: Sealed {}

trait Sealed {}

impl Sealed for Context {}
impl Sealed for PageFaultInterruptContext {}
impl Sealed for InterruptContext {}
impl Sealed for InterruptCodeContext {}

impl ProcessorContext for Context {}
impl ProcessorContext for PageFaultInterruptContext {}
impl ProcessorContext for InterruptContext {}
impl ProcessorContext for InterruptCodeContext {}
