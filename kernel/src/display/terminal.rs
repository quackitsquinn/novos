use core::{
    f32,
    fmt::{Debug, Write},
    mem,
    ops::{Index, IndexMut},
    ptr,
};

use alloc::{vec, vec::Vec};

use crate::{display::CURSOR_SPRITE, interrupts::without_interrupts, sdbg, sprintln, terminal};

use super::{FRAMEBUFFER, TERMINAL, color::Color, get_char, screen_char::ScreenChar};

const CHARACTER_BASE_SIZE: usize = 8;
// TODO: Replace derive(Debug) with a custom implementation that doesn't print the whole buffer
#[derive(Debug, Clone)]
pub struct Terminal {
    // x, y -> row, column
    front: CharacterVec,
    back: CharacterVec,
    position: (usize, usize),
    size: (usize, usize),
    character_size: (usize, usize),
    character_scale: usize,
    current_fg: Color,
    current_bg: Color,
    last_cursor: (usize, usize),
}

impl Terminal {
    pub const BLINK_CHAR: char = '_';
    pub const BLINK_CHAR_SCREEN_CHAR: ScreenChar =
        ScreenChar::new(Self::BLINK_CHAR, Color::new(230, 230, 230), Color::BLACK);

    pub fn new(scale: usize, line_spacing: usize) -> Self {
        let (term_size, glyph_size) = Self::calculate_dimensions(line_spacing, scale);
        sprintln!("Creating front buffer with size: {:?}", term_size);
        let front = vec![vec![ScreenChar::default(); term_size.0 as usize]; term_size.1 as usize];
        sprintln!("Creating back buffer with size: {:?}", term_size);
        let back = front.clone();
        Self {
            front: CharacterVec(front),
            back: CharacterVec(back),
            position: (0, 0),
            last_cursor: (0, 0),
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
        self.position.0 += 1;

        if self.position.0 >= self.size.0 {
            self.newline();
        }
        self.position
    }

    fn peak_advance(&self) -> (usize, usize) {
        let position = self.position.0 + 1;

        if self.position.0 >= self.size.0 {
            // Would scroll up, stay on the same line.
            if self.position.1 + 1 >= self.size.1 {
                return (0, self.position.1);
            }
            return (0, self.position.1 + 1);
        }

        (position, self.position.1)
    }

    fn newline(&mut self) {
        self.position.0 = 0;
        self.position.1 += 1;

        // Clear the last cursor if it exists.
        let (last_x, last_y) = self.last_cursor;
        if self.back[(last_x, last_y)] == Self::BLINK_CHAR_SCREEN_CHAR {
            self.set_char_at(last_x, last_y, ScreenChar::default());
        }

        if self.position.1 >= self.size.1 {
            self.scroll_up();
        }
    }

    #[optimize(speed)]
    fn scroll_up(&mut self) {
        self.position.1 -= 1;
        // Scroll up by one line
        self.back.0.remove(0);
        self.back
            .0
            .push(vec![ScreenChar::default(); self.size.0 as usize]);

        self.blit_update();
    }

    pub fn push_char(&mut self, c: char, flush: bool) {
        if c == '\n' {
            self.newline();
            return;
        }

        let pos = self.advance_cursor();
        self.back[pos] = ScreenChar::new(c, self.current_fg, self.current_bg);
        if flush {
            self.blit_flush_char(pos);
        }
    }

    fn offset(&self, pos: (usize, usize)) -> (usize, usize) {
        let (char_width, char_height) = self.character_size;
        (pos.0 * char_width, pos.1 * char_height)
    }

    #[inline]
    #[optimize(speed)]
    fn flush_character(&self, pos: (usize, usize)) {
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
    #[optimize(speed)]
    fn draw_over_character(&self, pos: (usize, usize), c: [u8; 8], color: Color) {
        let (x, y) = self.offset(pos);
        let mut fb = FRAMEBUFFER.get();
        fb.draw_sprite_transparent(x, y, self.character_scale, &c, color);
    }

    pub fn force_flush(&self) {
        for y in 0..self.size.1 {
            for x in 0..self.size.0 {
                self.flush_character((x, y));
            }
        }
    }

    #[inline]
    #[optimize(speed)]
    fn blit_update(&mut self) {
        for x in 0..self.size.0 {
            for y in 0..self.size.1 {
                if self.front[(x, y)] != self.back[(x, y)] {
                    self.front[(x, y)] = self.back[(x, y)];
                    self.flush_character((x, y));
                }
            }
        }
    }

    fn blit_flush_char(&mut self, pos: (usize, usize)) {
        if self.back[pos] == self.front[pos] {
            return;
        }
        self.front[pos] = self.back[pos];
        self.flush_character(pos);
    }

    pub fn backspace(&mut self) {
        if self.position.0 == 0 {
            return;
        }

        self.back[self.position] = ScreenChar::default();
        self.blit_flush_char(self.position);

        self.position.0 -= 1;
    }

    pub fn clear(&mut self) {
        self.back
            .0
            .iter_mut()
            .for_each(|v| v.fill(ScreenChar::default()));

        self.blit_update();
        self.position = (0, 0);
    }

    pub fn push_str(&mut self, s: &str) {
        //sprintln!("Pushing string: {}", s);
        for c in s.chars() {
            self.push_char(c, true);
        }
    }

    pub fn set_char_at(&mut self, x: usize, y: usize, c: ScreenChar) {
        self.back[(x, y)] = c;
        self.blit_flush_char((x, y));
    }

    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.position = (x, y);
    }

    pub fn blink_cursor(&mut self, fill: bool) {
        let (x, y) = self.peak_advance();

        // Clear the last cursor if it exists.
        if (x, y) != self.last_cursor {
            let (last_x, last_y) = self.last_cursor;
            if !(self.back[self.last_cursor] != Self::BLINK_CHAR_SCREEN_CHAR) {
                self.set_char_at(last_x, last_y, ScreenChar::default());
            } else {
                self.flush_character((last_x, last_y));
            }
        }

        self.last_cursor = (x, y);

        if fill {
            self.draw_over_character((x, y), CURSOR_SPRITE, Color::new(200, 200, 200));
            return;
        }
        self.flush_character((x, y));
    }

    pub fn get_size(&self) -> (usize, usize) {
        self.size
    }

    pub fn set_col(&mut self, col: usize) {
        (col..self.size.0).for_each(|c| {
            self.back[(c, self.position.1)] = ScreenChar::default();
            self.blit_flush_char((c, self.position.1));
        });
        self.position.0 = col;
    }

    pub fn update_row(&mut self) {
        for x in 0..self.size.0 {
            self.blit_flush_char((x, self.position.1));
        }
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.position
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

// TODO: This whole section should be refactored

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    without_interrupts(|| {
        crate::serial::interface::_print(args);
        if super::is_initialized() && !TERMINAL.is_locked() {
            write!(*terminal!(), "{}", args).unwrap();
        }
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::display::terminal::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(concat!($fmt, "\n"), $($arg)*));
}

// Port of the std dbg! macro, but it works in a no_std environment.
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
