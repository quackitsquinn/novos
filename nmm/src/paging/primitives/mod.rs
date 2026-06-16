pub mod frame;
pub mod paddr;
pub mod page;
pub mod vaddr;

use cake::encapsulate_macro;
pub use frame::{Frame, UnsizedFrame};
pub use paddr::PhysAddr;
pub use page::{Page, UnsizedPage};
pub use vaddr::VirtAddr;

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
