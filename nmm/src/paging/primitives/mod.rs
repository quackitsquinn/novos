pub mod address;
pub mod frame;
pub mod paddr;
pub mod page;
pub mod vaddr;

pub use address::{Address, AddressExt};
use cake::encapsulate_macro;
pub use frame::Frame;
pub use paddr::PhysAddr;
pub use page::Page;
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
pub trait PrimitiveSize: NmmSealed + Sized + Copy + core::fmt::Debug + Eq + PartialEq {
    /// The size of a page for this page size type, in bytes.
    const SIZE: u64;
    /// The name of this page size type, as a string.
    const NAME: &'static str;
}

/// Marker type for small pages, typically 4KB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Small;
impl PrimitiveSize for Small {
    const SIZE: u64 = crate::arch::L1_PAGE_SIZE;
    const NAME: &'static str = "Small";
}
/// Marker type for medium pages, typically 2MB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Medium;
impl PrimitiveSize for Medium {
    const SIZE: u64 = crate::arch::L2_PAGE_SIZE;
    const NAME: &'static str = "Medium";
}
/// Marker type for large pages, typically 1GB in size for x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Large;
impl PrimitiveSize for Large {
    const SIZE: u64 = crate::arch::L3_PAGE_SIZE;
    const NAME: &'static str = "Large";
}

/// A memory primitive.
#[allow(private_bounds)] // intentionally seal this
pub trait Primitive: NmmSealed + Sized + Copy + core::fmt::Debug + Eq + PartialEq {}

seal!(Small, Medium, Large);

/// A trait that represents both Page and Frame types, allowing for generic functions that can work with either type of memory primitive.
#[allow(private_bounds)] // intentionally seal this
pub const trait MemoryFragment<Ps: PrimitiveSize>: Primitive {
    /// The address space type associated with this memory primitive (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    type AddressType: Address;

    /// Tries to create a new memory primitive from the given starting address.
    /// The address must be aligned to the size of the primitive, otherwise this function will return `None`.
    fn from_start_address(start_address: Self::AddressType) -> Option<Self>;

    /// Creates a new memory primitive containing the given address.
    /// The starting address of the primitive will be the largest aligned address less than or equal to the given address.
    fn containing_address(addr: Self::AddressType) -> Self;

    /// Returns the starting address of this memory primitive as the appropriate address space type (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    fn start_address(&self) -> Self::AddressType;
}

/// A trait representing a family of memory fragments (e.g., pages or frames) that can be used in paging,
///  where each primitive has an associated address space type (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
#[allow(private_bounds)] // intentionally seal this
pub const trait PrimitiveClass:
    NmmSealed + Sized + Copy + core::fmt::Debug + Eq + PartialEq
{
    /// The address space type associated with this family of memory fragments (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    type Addr: Address;

    /// The memory fragments type associated with this family of memory fragments (e.g., `Page<S>` for pages, `Frame<S>` for frames).
    type Fragment<S: PrimitiveSize>: MemoryFragment<S, AddressType = Self::Addr> + Primitive;
}

/// The primitives used for virtual addresses and pages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PageClass;
impl PrimitiveClass for PageClass {
    type Addr = VirtAddr;
    type Fragment<S: PrimitiveSize> = Page<S>;
}

/// The primitives used for physical addresses and frames.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FrameClass;
impl PrimitiveClass for FrameClass {
    type Addr = PhysAddr;
    type Fragment<S: PrimitiveSize> = Frame<S>;
}

seal!(PageClass, FrameClass);

/// A memory primitive of unknown size.
/// This is used for functions that need to work with memory primitives of any size, but don't need to know the specific size of the primitive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnyPrimitive<C>
where
    C: PrimitiveClass,
{
    /// A small memory primitive, typically 4KB in size for x86_64 architecture.
    Small(C::Fragment<Small>),
    /// A medium memory primitive, typically 2MB in size for x86_64 architecture.
    Medium(C::Fragment<Medium>),
    /// A large memory primitive, typically 1GB in size for x86_64 architecture.
    Large(C::Fragment<Large>),
}

impl<C> AnyPrimitive<C>
where
    C: PrimitiveClass,
{
    /// Returns the starting address of this memory primitive as the appropriate address space type (e.g., `VirtAddr` for pages, `PhysAddr` for frames).
    pub fn start_address(&self) -> C::Addr {
        match self {
            AnyPrimitive::Small(prim) => prim.start_address(),
            AnyPrimitive::Medium(prim) => prim.start_address(),
            AnyPrimitive::Large(prim) => prim.start_address(),
        }
    }

    /// Returns the size of this memory primitive in bytes.
    pub fn size(&self) -> u64 {
        match self {
            AnyPrimitive::Small(_) => Small::SIZE,
            AnyPrimitive::Medium(_) => Medium::SIZE,
            AnyPrimitive::Large(_) => Large::SIZE,
        }
    }
}

/// Type alias for a memory primitive of unknown size that is specifically a page.
pub type AnyPage = AnyPrimitive<PageClass>;
/// Type alias for a memory primitive of unknown size that is specifically a frame.
pub type AnyFrame = AnyPrimitive<FrameClass>;
