use core::{ops::Deref, slice};

use bytemuck::{Pod, Zeroable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedNulString<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> FixedNulString<N> {
    pub fn new() -> Self {
        Self { data: [0; N] }
    }

    pub const fn from_str(s: &str) -> Option<Self> {
        if s.len() >= N {
            return None;
        }

        let mut data = [0; N];
        // SAFETY: We can't call `bytes()` because it is not const, so we have to do this ugly thing.
        let slice_data = unsafe { slice::from_raw_parts(s.as_ptr(), s.len()) };
        let mut i = 0;
        while i < s.len() {
            data[i] = slice_data[i];
            i += 1;
        }
        Some(Self { data })
    }
}

unsafe impl<const N: usize> Zeroable for FixedNulString<N> {}
unsafe impl<const N: usize> Pod for FixedNulString<N> {}

impl<const N: usize> Deref for FixedNulString<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        let len = self.data.iter().position(|&c| c == 0).unwrap_or(N);
        core::str::from_utf8(&self.data[..len]).unwrap()
    }
}

impl<const N: usize> Validate for FixedNulString<N> {
    fn validate(&self) -> bool {
        true
    }
}

mod null_macro {
    macro_rules! null_str {
    ($cap: expr) => {
        $crate::common::fixed_null_str::FixedNulString<{$cap}>
    };}
    pub(crate) use null_str;
}

pub(crate) use null_macro::null_str;

use super::validate::Validate;
