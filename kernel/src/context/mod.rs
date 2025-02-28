mod contexts;
mod int_context;

use core::fmt::write;
use core::fmt::Debug;
use core::fmt::Display;
use core::ops::Deref;

pub use contexts::ContextValue;
pub use contexts::PageFaultInterruptContextValue;
pub use int_context::InterruptCodeContextValue;
pub use int_context::InterruptContextValue;

#[allow(private_bounds)] // Don't let implementations on arbitrary types
pub trait ProcessorContext: Sealed {}

trait Sealed {}

impl Sealed for ContextValue {}
impl Sealed for PageFaultInterruptContextValue {}
impl Sealed for InterruptContextValue {}
impl Sealed for InterruptCodeContextValue {}

impl ProcessorContext for ContextValue {}
impl ProcessorContext for PageFaultInterruptContextValue {}
impl ProcessorContext for InterruptContextValue {}
impl ProcessorContext for InterruptCodeContextValue {}

// Keeping *mut T private is intentional so that this cannot be constructed anywhere.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Context<T: ProcessorContext>(*mut T);

pub type InterruptContext = Context<InterruptContextValue>;
pub type PageFaultInterruptContext = Context<PageFaultInterruptContextValue>;
pub type InterruptCodeContext = Context<InterruptCodeContextValue>;

impl<T: ProcessorContext> Deref for Context<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T: ProcessorContext> Context<T> {
    pub unsafe fn modify(&self) -> &mut T {
        unsafe { &mut *self.0 }
    }
}

impl<T: ProcessorContext> Display for Context<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", &*self)
    }
}

impl<T: ProcessorContext> Debug for Context<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", &*self)
    }
}
