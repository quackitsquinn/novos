use core::ops::Deref;

use bytemuck::{Pod, Zeroable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedNulString<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> FixedNulString<N> {
    pub fn new() -> Self {
        Self { data: [0; N] }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() >= N {
            return None;
        }

        let mut data = [0; N];
        data[..s.len()].copy_from_slice(s.as_bytes());
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
