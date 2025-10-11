//! Terminal module for handling text output to the screen.
use core::{
    f32,
    fmt::{Debug, Write},
    ops::{Index, IndexMut},
};

use alloc::{vec, vec::Vec};

use crate::{interrupts::without_interrupts, sprintln};

use super::{FRAMEBUFFER, TERMINAL, color::Color, get_char, screen_char::ScreenChar};

const CHARACTER_BASE_SIZE: usize = 8;

/// A terminal for displaying text.
#[derive(Debug, Clone)]
pub struct Terminal {
    // x, y -> row, column
    front: CharacterVec,
    back: CharacterVec,
    cursor: (usize, usize),
    size: (usize, usize),
    character_size: (usize, usize),
    character_scale: usize,
    /// The current foreground color.
    pub current_fg: Color,
    /// The current background color.
    pub current_bg: Color,
}

impl Terminal {
    pub(super) fn new(scale: usize, line_spacing: usize) -> Self {
        let (term_size, glyph_size) = Self::calculate_dimensions(line_spacing, scale);
        sprintln!("Creating front buffer with size: {:?}", term_size);
        let front = vec![vec![ScreenChar::default(); term_size.0 as usize]; term_size.1 as usize];
        sprintln!("Creating back buffer with size: {:?}", term_size);
        let back = front.clone();
        Self {
            front: CharacterVec(front),
            back: CharacterVec(back),
            cursor: (0, 0),
            size: term_size,
            character_size: glyph_size,
            character_scale: scale,
            current_fg: Color::new(255, 255, 255),
            current_bg: Color::new(0, 0, 0),
        }
    }

    fn calculate_dimensions(line_spacing: usize, scale: usize) -> ((usize, usize), (usize, usize)) {
        let fb = FRAMEBUFFER.get();
        let (width, height) = fb.size();
        let char_width = CHARACTER_BASE_SIZE * scale;
        let char_height = CHARACTER_BASE_SIZE * scale + line_spacing;
        let width = width as f64 / char_width as f64;
        let height = height as f32 / char_height as f32;
        // For some reason, there are no math functions for either f32 or f64, probably due to no std.
        // So we have to do this manually.
        let width_floor = width - (width % 1.0);
        let height_floor = height - (height % 1.0);

        (
            (width_floor as usize, height_floor as usize),
            (char_width as usize, char_height as usize),
        )
    }

    fn advance_cursor(&mut self) -> (usize, usize) {
        if self.cursor.0 >= self.size.0 - 1 {
            self.newline();
            return self.cursor;
        }
        self.cursor.0 += 1;
        self.cursor
    }

    fn newline(&mut self) {
        self.cursor.0 = 0;
        self.cursor.1 += 1;

        if self.cursor.1 >= self.size.1 {
            self.scroll_up();
        }
    }

    fn scroll_up(&mut self) {
        self.cursor.1 -= 1;
        // Scroll up by one line
        self.back.0.remove(0);
        self.back
            .0
            .push(vec![ScreenChar::default(); self.size.0 as usize]);

        self.flush();
    }

    /// Push a character to the terminal.
    pub fn push_char(&mut self, c: char, flush: bool) {
        if c == '\n' {
            self.newline();
            return;
        }

        let pos = self.advance_cursor();
        self.back[pos] = ScreenChar::new(c, self.current_fg, self.current_bg);
        if flush {
            self.flush_char(pos);
        }
    }

    fn offset(&self, pos: (usize, usize)) -> (usize, usize) {
        let (char_width, char_height) = self.character_size;
        (pos.0 * char_width, pos.1 * char_height)
    }

    #[inline]
    fn draw_character(&self, pos: (usize, usize)) {
        let c = self.front[pos];
        let (x, y) = self.offset(pos);
        let mut fb = FRAMEBUFFER.get();
        fb.draw_sprite(
            x,
            y,
            self.character_scale,
            &get_char(c.character),
            c.foreground,
            c.background,
        );
    }

    /// Flushes the whole screen, ignoring any optimizations.
    pub fn force_flush(&self) {
        for y in 0..self.size.1 {
            for x in 0..self.size.0 {
                self.draw_character((x, y));
            }
        }
    }

    /// Flushes the whole screen, only updating changed characters.
    #[inline]
    pub fn flush(&mut self) {
        for x in 0..self.size.0 {
            for y in 0..self.size.1 {
                if self.front[(x, y)] != self.back[(x, y)] {
                    self.front[(x, y)] = self.back[(x, y)];
                    self.draw_character((x, y));
                }
            }
        }
    }

    fn flush_char(&mut self, pos: (usize, usize)) {
        if self.back[pos] == self.front[pos] {
            return;
        }
        self.front[pos] = self.back[pos];
        self.draw_character(pos);
    }

    /// Removes the last character from the terminal.
    pub fn backspace(&mut self) {
        if self.cursor.0 == 0 {
            return;
        }

        self.back[self.cursor] = ScreenChar::default();
        self.flush_char(self.cursor);

        self.cursor.0 -= 1;
    }

    /// Clears the terminal.
    pub fn clear(&mut self) {
        self.back
            .0
            .iter_mut()
            .for_each(|v| v.fill(ScreenChar::default()));

        self.flush();
        self.cursor = (0, 0);
    }

    /// Push a string to the terminal.
    pub fn push_str(&mut self, s: &str) {
        //sprintln!("Pushing string: {}", s);
        for c in s.chars() {
            self.push_char(c, true);
        }
    }

    /// Sets the current column.
    pub fn set_col(&mut self, col: usize) {
        (col..self.size.0).for_each(|c| {
            self.back[(c, self.cursor.1)] = ScreenChar::default();
            self.flush_char((c, self.cursor.1));
        });
        self.cursor.0 = col;
    }

    /// Updates the current row.
    pub fn update_row(&mut self) {
        for x in 0..self.size.0 {
            self.flush_char((x, self.cursor.1));
        }
    }
}

impl Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

/// A Vector that swaps it's indices.
/// This struct provides a massive speedup when shifting the display up by swapping the order of x and y.
/// This can make the code more difficult to read because of the swapped (y, x)
#[derive(Clone)]
struct CharacterVec(Vec<Vec<ScreenChar>>);

impl Index<(usize, usize)> for CharacterVec {
    type Output = ScreenChar;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.0[index.1][index.0]
    }
}

impl IndexMut<(usize, usize)> for CharacterVec {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        &mut self.0[index.1][index.0]
    }
}

impl Debug for CharacterVec {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("CharacterVec").finish()
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    without_interrupts(|| {
        crate::serial::interface::_print(args);
        if super::is_initialized()
            && let Some(mut terminal) = TERMINAL.try_get()
        {
            write!(*terminal, "{}", args).unwrap();
        }
    });
}

/// Prints to the terminal. Same functionality as the standard print! macro.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::display::terminal::_print(format_args!($($arg)*)));
}

/// Prints to the terminal, appending a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

/// Prints the given value and its source location then returns the value.
#[macro_export]
macro_rules! dbg {
    () => {
        $crate::println!("[{}:{}:{}]", core::file!(), core::line!(), core::column!());
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::println!("[{}:{}:{}] {} = {:#?}",
                    core::file!(), core::line!(), core::column!(),
                    core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}
