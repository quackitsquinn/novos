use core::fmt::Write;

use arrayvec::ArrayVec;

use crate::{framebuffer, terminal};

use super::{color::Color, screen_char::ScreenChar};

const MAX_HEIGHT: usize = 32;
const MAX_WIDTH: usize = MAX_HEIGHT * 2;
pub struct Terminal {
    // x, y -> row, column
    chars: ArrayVec<ArrayVec<ScreenChar, MAX_HEIGHT>, MAX_WIDTH>,
    position: (u32, u32),
    char_size: (u32, u32),
    size: (usize, usize),
    scale: u32,
}

impl Terminal {
    pub fn new() -> Self {
        //   sprintln!("Getting fb size");
        let size = framebuffer!().size();
        let char_size = ((size.0 / 8) as u32, (size.1 / 8) as u32);
        let mut s = Self {
            chars: Self::make_vec(char_size),
            position: (0, 0),
            char_size,
            size,
            scale: 1,
        };
        // sprintln!("Setting scale");
        // Default to 2x scale because 90% of the time 8px is too small
        s.set_scale(2);
        s
    }

    fn make_vec(dim: (u32, u32)) -> ArrayVec<ArrayVec<ScreenChar, MAX_HEIGHT>, MAX_WIDTH> {
        let mut vec: ArrayVec<ArrayVec<ScreenChar, MAX_HEIGHT>, MAX_WIDTH> = ArrayVec::new();
        unsafe { vec.set_len(vec.capacity()) };
        for i in 0..vec.capacity() {
            let mut v = ArrayVec::new();
            unsafe { v.set_len(v.capacity()) };
            // We don't need to initialize the array because the initial state is already zeroed
            vec[i] = v;
        }
        vec
    }
    /// Set the scale of the terminal.
    pub fn set_scale(&mut self, scale: u32) {
        self.scale = scale;
        let old = self.char_size;

        self.set_char_size((
            (self.size.1 / scale as usize * 8) as u32,
            (self.size.0 / scale as usize * 8) as u32,
        ));

        unsafe {
            self.chars.set_len(self.char_size.0 as usize);
        }

        for row in self.chars.iter_mut() {
            unsafe { row.set_len(self.char_size.1 as usize) };
        }
    }

    fn set_char_size(&mut self, size: (u32, u32)) {
        let size = (size.0.min(MAX_WIDTH as u32), size.1.min(MAX_HEIGHT as u32));
        self.char_size = size;
    }

    pub fn set_position(&mut self, x: u32, y: u32) {
        self.position = (x, y);
    }

    pub fn shift_up(&mut self) {
        self.chars.remove(0);
        self.chars.push(ArrayVec::new());
        self.draw_all();
    }

    pub fn push_char(&mut self, c: char, color: Color) {
        //sprintln!("Pushing char: {}", c);
        if c == '\n' {
            self.newline();
            return;
        }
        if c == '\r' {
            self.position.1 = 0;
            return;
        }
        if c == '\t' {
            self.position.1 += 4;
            return;
        }

        self.chars[self.position.0 as usize][self.position.1 as usize] = ScreenChar::new(c, color);
        self.draw_char(self.position.0, self.position.1);
        self.update_cursor();
    }

    pub fn push_str(&mut self, s: &str, color: Color) {
        //sprintln!("Pushing string: {}", s);
        for c in s.chars() {
            self.push_char(c, color);
        }
    }

    fn update_cursor(&mut self) {
        if self.position.0 + 1 >= self.char_size.0 {
            self.newline();
        } else {
            self.position.0 += 1;
        }
    }

    fn newline(&mut self) {
        if self.position.1 + 1 >= self.char_size.1 {
            self.shift_up();
        } else {
            self.position.1 += 1;
        }
        self.position.0 = 0;
    }

    pub fn clear(&mut self) {
        self.chars = Self::make_vec(self.char_size);
        self.position = (0, 0);
        self.set_scale(self.scale);
    }

    fn draw_char(&self, x: u32, y: u32) {
        let buf_char = self.chars[x as usize][y as usize];
        let charac = buf_char.character();
        let sprite = super::get_char(charac);
        framebuffer!().draw_scaled_sprite(
            x as usize * 8 * self.scale as usize,
            y as usize * 8 * self.scale as usize,
            self.scale as usize,
            &sprite,
            buf_char.foreground(),
            buf_char.background(),
        );
    }

    fn draw_all(&self) {
        for (i, row) in self.chars.iter().enumerate() {
            for (j, _) in row.iter().enumerate() {
                self.draw_char(i as u32, j as u32);
            }
        }
    }
}

impl Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.push_str(s, Color::new(255, 255, 255));
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    crate::serial::interface::_print(args);
    if crate::display_init() {
        write!(*terminal!(), "{}", args).unwrap();
    }
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
