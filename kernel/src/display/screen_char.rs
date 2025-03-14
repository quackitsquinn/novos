use super::color::Color;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenChar {
    character: char,
    foreground: Color,
    background: Color,
}

impl ScreenChar {
    pub const fn new(character: char, fg: Color, bg: Color) -> Self {
        Self {
            character,
            foreground: fg,
            background: bg,
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

impl Default for ScreenChar {
    fn default() -> Self {
        Self::new(' ', Color::new(255, 255, 255), Color::new(0, 0, 0))
    }
}
