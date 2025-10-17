/// A trait for validating data structures.
pub trait Validate {
    /// Validate the data structure.
    fn validate(&self) -> bool;
}
macro_rules! always_validate {
    ($name:ident) => {
        impl Validate for $name {
            fn validate(&self) -> bool {
                true
            }
        }
    };
    ($($name:ident),+) => {
        $(
            always_validate!($name);
        )+
    };
}

always_validate!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
always_validate!(usize, isize);
always_validate!(u128, i128);

impl<T, const N: usize> Validate for [T; N]
where
    T: Validate,
{
    fn validate(&self) -> bool {
        self.iter().all(|x| x.validate())
    }
}

impl<T: Validate> Validate for &[T] {
    fn validate(&self) -> bool {
        self.iter().all(|x| x.validate())
    }
}
