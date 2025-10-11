//! A character on the screen, including its foreground and background colors.
use super::color::Color;
/// A character on the screen, including its foreground and background colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenChar {
    /// The character to display.
    pub character: char,
    /// The foreground color of the character.
    pub foreground: Color,
    /// The background color of the character.
    pub background: Color,
}

impl ScreenChar {
    /// Create a new screen character.
    pub const fn new(character: char, fg: Color, bg: Color) -> Self {
        Self {
            character,
            foreground: fg,
            background: bg,
        }
    }
}

impl Default for ScreenChar {
    /// Create a default screen character (an empty character with white foreground and black background).
    fn default() -> Self {
        Self::new('\0', Color::new(255, 255, 255), Color::new(0, 0, 0))
    }
}
