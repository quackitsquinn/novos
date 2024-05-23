#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Create a new color.
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert the color to a slice.
    #[inline] // This is a simple fn that gets called a *lot*. It's worth inlining.
    pub fn to_slice(&self, slice: &mut [u8]) {
        assert!(slice.len() >= 3);
        slice[0] = self.r;
        slice[1] = self.g;
        slice[2] = self.b;
    }
}
