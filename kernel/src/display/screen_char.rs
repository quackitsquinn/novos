use super::color::Color;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenChar {
    pub character: char,
    pub foreground: Color,
    pub background: Color,
}

impl ScreenChar {
    pub const fn new(character: char, fg: Color, bg: Color) -> Self {
        Self {
            character,
            foreground: fg,
            background: bg,
        }
    }
}

impl Default for ScreenChar {
    fn default() -> Self {
        Self::new(' ', Color::new(255, 255, 255), Color::new(0, 0, 0))
    }
}
