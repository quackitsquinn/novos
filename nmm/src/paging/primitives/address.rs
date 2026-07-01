//! Generic address type for the current architecture, used for both virtual and physical addresses.

use core::{marker::Destruct, ops};

use crate::paging::{FragmentSize, MemoryFragment, primitives::Primitive};

/// Helper trait to make the `Address definition a little less gross
const trait AddressMath:
    Sized
    + [const] ops::Add<u64, Output = Self>
    + [const] ops::Sub<u64, Output = Self>
    + [const] ops::AddAssign<u64>
    + [const] ops::SubAssign<u64>
    + [const] ops::Add<Self>
    + [const] ops::Sub<Self>
    + [const] ops::AddAssign<Self>
    + [const] ops::SubAssign<Self>
{
}

impl<T> AddressMath for T where
    T: Sized
        + ops::Add<u64, Output = Self>
        + ops::Sub<u64, Output = Self>
        + ops::AddAssign<u64>
        + ops::SubAssign<u64>
        + ops::Add<Self>
        + ops::Sub<Self>
        + ops::AddAssign<Self>
        + ops::SubAssign<Self>
{
}

/// Address space primitives, e.g. `VirtAddr` and `PhysAddr`.
///
/// This is used for generic functions that can work with either virtual or physical addresses.
#[allow(private_bounds)]
pub const trait Address: Primitive + Ord + PartialOrd + AddressMath {
    /// Tries to create a new address from the given value.
    /// The value must be valid for the current architecture's address, otherwise this function will return `None`.
    fn try_new(val: u64) -> Option<Self>;

    /// Creates a new address from the given value.
    /// The value must be valid for the current architecture's address, otherwise this function will panic.
    fn new(val: u64) -> Self {
        Self::try_new(val).expect("Address::new: value is invalid for this address")
    }

    /// Creates a new address from the given value, truncating any bits beyond the architecture's bit width.
    fn new_truncate(val: u64) -> Self;

    /// Creates a new address from the given value without checking for validity.
    unsafe fn new_unchecked(val: u64) -> Self;

    /// Creates a new address from the given memory primitive.
    ///  The starting address of the primitive will be used as the value for the address.
    fn from_primitive<P: [const] MemoryFragment<S> + [const] Destruct, S: FragmentSize>(
        primitive: P,
    ) -> Option<Self>
    where
        P::AddressType: [const] Address,
    {
        let primitive_addr = primitive.start_address();
        Self::try_new(primitive_addr.as_u64())
    }

    /// Adds `rhs` to this address, returning a new address if the result is valid for the current architecture's address, or `None` if the result is invalid.
    fn checked_add(&self, rhs: u64) -> Option<Self> {
        match self.as_u64().checked_add(rhs) {
            Some(val) => Self::try_new(val),
            None => None,
        }
    }

    /// Returns the value of this address as a `u64`.
    fn as_u64(&self) -> u64;
}

/// Non-const additions to `Address` types.
pub trait AddressExt: Address {
    /// Creates a new address using `ptr` as the value for the address.
    fn from_ptr<T>(ptr: *const T) -> Option<Self> {
        Self::try_new(ptr as u64)
    }

    /// Creates a new address using `ptr` as the value for the address.
    fn from_mut_ptr<T>(ptr: *mut T) -> Option<Self> {
        Self::try_new(ptr as u64)
    }

    /// Returns the value of this address as a pointer of the given type.
    ///
    /// The returned pointer is not guaranteed to be valid for dereferencing, and the caller must ensure that any dereferencing of the returned pointer is safe.
    fn as_ptr<T>(&self) -> *const T {
        (self.as_u64() as usize) as *const T
    }

    /// Returns the value of this address as a pointer of the given type.
    ///
    /// The returned pointer is not guaranteed to be valid for dereferencing, and the caller must ensure that any dereferencing of the returned pointer is safe.
    fn as_mut_ptr<T>(&self) -> *mut T {
        (self.as_u64() as usize) as *mut T
    }
}

impl<T: Address> AddressExt for T {}
