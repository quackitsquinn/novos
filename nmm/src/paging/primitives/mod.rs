pub mod frame;
pub mod paddr;
pub mod page;
pub mod vaddr;

use core::{marker::Destruct, ops};

use cake::encapsulate_macro;
pub use frame::{Frame, UnsizedFrame};
pub use paddr::PhysAddr;
pub use page::{Page, UnsizedPage};
pub use vaddr::VirtAddr;

use crate::{NmmSealed, seal};

encapsulate_macro!(
    impl_ops,
    _impl_op_mod,
    macro_rules! impl_ops {
    (single $op: tt, $op_trait: ident, $op_fn_name: ident, $newtype: ident ) => {
        impl ops::$op_trait<u64> for $newtype {
            type Output = Self;
            fn $op_fn_name(self, rhs: u64) -> Self {
                Self(self.0.$op_fn_name(rhs))
            }
        }

        impl ops::$op_trait<Self> for $newtype {
            type Output = Self;
            fn $op_fn_name(self, rhs: Self) -> Self {
                Self(self.0.$op_fn_name(rhs.0))
            }
        }
    };

    (assign $op: tt, $op_trait: ident, $op_fn_name: ident, $newtype: ident) => {
        impl ops::$op_trait<u64> for $newtype {
            fn $op_fn_name(&mut self, rhs: u64) {
                self.0.$op_fn_name(rhs);
            }
        }

        impl ops::$op_trait<Self> for $newtype {
            fn $op_fn_name(&mut self, rhs: Self) {
                self.0.$op_fn_name(rhs.0);
            }
        }
    };

    (blanket $newtype: ident) => {
        impl_ops!(single Add, Add, add, $newtype);
        impl_ops!(single Sub, Sub, sub, $newtype);
        impl_ops!(assign AddAssign, AddAssign, add_assign, $newtype);
        impl_ops!(assign SubAssign, SubAssign, sub_assign, $newtype);
    };
}
);

/// A trait representing a page size for the current architecture.
#[allow(private_bounds)]
pub trait PrimitiveSize: NmmSealed {
    /// The size of a page for this page size type, in bytes.
    const SIZE: u64;
}

/// Marker type for small pages, typically 4KB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Small;
impl PrimitiveSize for Small {
    const SIZE: u64 = crate::arch::L1_PAGE_SIZE;
}
/// Marker type for medium pages, typically 2MB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Medium;
impl PrimitiveSize for Medium {
    const SIZE: u64 = crate::arch::L2_PAGE_SIZE;
}
/// Marker type for large pages, typically 1GB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Large;
impl PrimitiveSize for Large {
    const SIZE: u64 = crate::arch::L3_PAGE_SIZE;
}

seal!(Small, Medium, Large);

/// A trait representing a memory primitive that can be used in paging, such as a page or a frame.
/// This trait is sealed to prevent external implementations, ensuring that only the intended types (like `Page` and `Frame`) can be used as memory primitives
/// in the paging system.
#[allow(private_bounds)] // intentionally seal this
pub const trait MemoryPrimitive<Ps: PrimitiveSize>: NmmSealed {
    /// The address space type associated with this memory primitive (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    type AddressSpace: Address;

    /// Returns the starting address of this memory primitive as the appropriate address space type (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    fn start_address(&self) -> Self::AddressSpace;
}

/// Helper trait to make AddressSpace's definition a little less gross
const trait AddrSpaceMath:
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

impl<T> AddrSpaceMath for T where
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
pub const trait Address:
    NmmSealed + Copy + core::fmt::Debug + Eq + PartialEq + Ord + PartialOrd + AddrSpaceMath
{
    /// Tries to create a new address from the given value.
    /// The value must be valid for the current architecture's address, otherwise this function will return `None`.
    fn try_new(val: u64) -> Option<Self>;

    /// Creates a new address from the given value.
    /// The value must be valid for the current architecture's address, otherwise this function will panic.
    fn new(val: u64) -> Self {
        Self::try_new(val).expect("AddressSpace::new: value is invalid for this address")
    }

    /// Creates a new address from the given value, truncating any bits beyond the architecture's bit width.
    fn new_truncate(val: u64) -> Self;

    /// Creates a new address from the given value without checking for validity.
    unsafe fn new_unchecked(val: u64) -> Self;

    /// Creates a new address from the given memory primitive.
    ///  The starting address of the primitive will be used as the value for the address.
    fn from_primitive<P: [const] MemoryPrimitive<S> + [const] Destruct, S: PrimitiveSize>(
        primitive: P,
    ) -> Option<Self>
    where
        P::AddressSpace: [const] Address,
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
