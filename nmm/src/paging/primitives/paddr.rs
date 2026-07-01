//! Physical address type for the current architecture.
use crate::{
    arch,
    paging::{
        Address,
        primitives::{Primitive, impl_ops},
    },
    seal,
};

use core::ops;

/// Physical address type for the current architecture.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PhysAddr(u64);

seal!(PhysAddr);

impl_ops!(blanket PhysAddr);

impl Primitive for PhysAddr {}

const impl Address for PhysAddr {
    fn try_new(val: u64) -> Option<Self> {
        if arch::is_valid_phys(val) {
            Some(PhysAddr(val))
        } else {
            None
        }
    }

    unsafe fn new_unchecked(val: u64) -> Self {
        PhysAddr(val)
    }

    fn new_truncate(val: u64) -> Self {
        Self(arch::canonicalize_phys(val))
    }

    fn as_u64(&self) -> u64 {
        self.0
    }
}
