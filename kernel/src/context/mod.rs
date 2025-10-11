#![allow(private_bounds)] // Don't let implementations on arbitrary types
mod contexts;
mod int_context;

use core::fmt::Debug;
use core::fmt::Display;
use core::ops::Deref;

pub use contexts::ContextValue;
pub use contexts::PageFaultInterruptContextValue;
pub use int_context::InterruptCodeContextValue;
pub use int_context::InterruptContextValue;

/// A trait representing a processor context.
pub trait ProcessorContext: Sealed {}

/// A trait to prevent external implementations.
trait Sealed {}

impl Sealed for ContextValue {}
impl Sealed for PageFaultInterruptContextValue {}
impl Sealed for InterruptContextValue {}
impl Sealed for InterruptCodeContextValue {}

impl ProcessorContext for ContextValue {}
impl ProcessorContext for PageFaultInterruptContextValue {}
impl ProcessorContext for InterruptContextValue {}
impl ProcessorContext for InterruptCodeContextValue {}

/// A context representing the state of a processor at a given point in time.
#[repr(transparent)]
#[derive(Clone, Copy)]
// Keeping *mut T private is intentional so that this cannot be constructed anywhere.
pub struct Context<T: ProcessorContext>(*mut T);

/// A context representing the state of an interrupt.
pub type InterruptContext = Context<InterruptContextValue>;
/// A context representing the state of a page fault interrupt.
pub type PageFaultInterruptContext = Context<PageFaultInterruptContextValue>;
/// A context representing the state of an interrupt with an associated error code.
pub type InterruptCodeContext = Context<InterruptCodeContextValue>;

impl<T: ProcessorContext> Deref for Context<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T: ProcessorContext> Context<T> {
    /// Returns a mutable reference to the context.
    /// # Safety
    /// The caller must ensure that the context is modified in a way that does not violate aliasing rules.
    pub unsafe fn modify(&mut self) -> &mut T {
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
