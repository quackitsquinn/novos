//! Architecture-specific types and implementations for the memory manager.
#[cfg(feature = "x86_64")]
pub mod x86_64;
#[cfg(feature = "x86_64")]
use x86_64 as arch_impl;

pub type PhysAddr = arch_impl::PhysAddr;
pub type VirtAddr = arch_impl::VirtAddr;
