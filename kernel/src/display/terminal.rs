use core::{f32, fmt::Write};

use alloc::{vec, vec::Vec};
use log::debug;

use crate::{
    framebuffer,
    interrupts::{disable, enable, without_interrupts},
    sprintln, terminal,
};

use super::{color::Color, get_char, screen_char::ScreenChar, FRAMEBUFFER, TERMINAL};

const CHARACTER_BASE_SIZE: usize = 8;
// TODO: Replace derive(Debug) with a custom implementation that doesn't print the whole buffer
#[derive(Debug, Clone)]
pub struct Terminal {
    // x, y -> row, column
    front: Vec<Vec<ScreenChar>>,
    back: Vec<Vec<ScreenChar>>,
    position: (usize, usize),
    size: (usize, usize),
    character_size: (usize, usize),
    character_scale: usize,
    current_fg: Color,
    current_bg: Color,
}

impl Terminal {
    pub fn new(scale: usize, line_spacing: usize) -> Self {
        let (term_size, glyph_size) = Self::calculate_dimensions(line_spacing, scale);
        sprintln!("Creating front buffer with size: {:?}", term_size);
        let front = vec![vec![ScreenChar::default(); term_size.0 as usize]; term_size.1 as usize];
        sprintln!("Creating back buffer with size: {:?}", term_size);
        let back = front.clone();
        Self {
            front,
            back,
            position: (0, 0),
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

    fn newline(&mut self) {
        self.position.0 = 0;
        self.position.1 += 1;
        if self.position.1 >= self.size.1 {
            self.scroll_up();
        }
    }

    fn scroll_up(&mut self) {
        // Copy the screen to the back buffer
        self.back.clone_from_slice(&self.front);
        // Scroll up by one line
        self.front.remove(0);
        self.front
            .push(vec![ScreenChar::default(); self.size.0 as usize]);
        self.position.1 -= 1;
        self.blit_update();
    }

    pub fn push_char(&mut self, c: char, flush: bool) {
        if c == '\n' {
            self.newline();
            return;
        }

        let (x, y) = self.advance_cursor();
        self.front[y][x] = ScreenChar::new(c, self.current_fg, self.current_bg);
        if flush {
            self.flush_character((x, y));
        }
    }

    fn offset(&self, pos: (usize, usize)) -> (usize, usize) {
        let (char_width, char_height) = self.character_size;
        (pos.0 * char_width, pos.1 * char_height)
    }

    #[inline]
    fn flush_character(&self, pos: (usize, usize)) {
        let c = self.front[pos.1][pos.0];
        let (x, y) = self.offset(pos);
        let mut fb = FRAMEBUFFER.get();
        fb.draw_sprite(
            x,
            y,
            self.character_scale,
            &get_char(c.character()),
            c.foreground(),
            c.background(),
        );
    }

    #[inline]
    fn blit_update(&self) {
        for y in 0..self.size.1 {
            let front = &self.front[y];
            let back = &self.back[y];
            for x in 0..self.size.0 {
                if front[x] != back[x] {
                    self.flush_character((x, y));
                }
            }
        }
    }

    pub fn push_str(&mut self, s: &str) {
        //sprintln!("Pushing string: {}", s);
        for c in s.chars() {
            self.push_char(c, true);
        }
    }
}

impl Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

// TODO: This whole section should be refactored

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    without_interrupts(|| {
        crate::serial::interface::_print(args);
        if super::is_initialized() {
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
