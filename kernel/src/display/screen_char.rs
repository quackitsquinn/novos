use super::color::Color;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenChar {
    character: char,
    foreground: Color,
    background: Color,
}

impl ScreenChar {
    pub const fn new(character: char, color: Color) -> Self {
        Self {
            character,
            foreground: color,
            background: Color::new(0, 0, 0),
        }
    }

    pub fn character(&self) -> char {
        self.character
    }

    pub fn foreground(&self) -> Color {
        self.foreground
    }

    pub fn background(&self) -> Color {
        self.background
    }
}
