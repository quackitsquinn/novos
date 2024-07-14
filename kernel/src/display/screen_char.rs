use super::color::Color;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenChar {
    character: char,
    color: Color,
}

impl ScreenChar {
    pub const fn new(character: char, color: Color) -> Self {
        Self { character, color }
    }

    pub fn character(&self) -> char {
        self.character
    }

    pub fn color(&self) -> Color {
        self.color
    }
}
