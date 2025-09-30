use core::{
    mem, panic,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{boxed::Box, string::String};
use arrayvec::ArrayVec;
use cake::Owned;
use pc_keyboard::{
    layouts::Us104Key, DecodedKey, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1,
};

use x86_64::instructions::port::Port;

use crate::{
    context::InterruptContext,
    interrupt_wrapper,
    interrupts::{
        hardware::{InterruptIndex, PICS},
        lock::InterruptMutex,
    },
};

pub static KEYBOARD: InterruptMutex<KeyboardDriver> =
    InterruptMutex::new(unsafe { KeyboardDriver::new() });

pub struct KeyboardDriver {
    line: ArrayVec<u8, 255>,
    raw: ArrayVec<u8, 255>,
    new_chars: usize,
    backspaces: usize,
    repr: Keyboard<Us104Key, ScancodeSet1>,
}

impl KeyboardDriver {
    /// Creates a new `Keyboard` instance.
    pub const unsafe fn new() -> Self {
        KeyboardDriver {
            line: ArrayVec::new_const(),
            raw: ArrayVec::new_const(),
            new_chars: 0,
            backspaces: 0,
            repr: Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore),
        }
    }

    pub(super) unsafe fn input(&mut self, chr: u8) {
        if chr == 0x08 || chr == 0x7f {
            self.backspaces += 1;
            self.line.pop();
            return;
        }
        if !self.line.is_full() {
            self.line.push(chr);
        }

        if !self.raw.is_full() {
            self.raw.push(chr);
            self.new_chars += 1;
        }
    }

    pub(super) unsafe fn scancode(&mut self, code: u8) {
        if let Ok(Some(key_event)) = self.repr.add_byte(code) {
            if key_event.state == KeyState::Up {
                // We don't handle key releases.
                // .. but the keyboard driver does.
                self.repr.process_keyevent(key_event);
                return;
            }
            if let Some(chr) = self.repr.process_keyevent(key_event) {
                match chr {
                    DecodedKey::RawKey(KeyCode::Return) => {
                        unsafe { self.input(b'\n') };
                    }
                    DecodedKey::Unicode(c) => {
                        if !c.is_ascii() {
                            panic!("Non-ASCII character received");
                        }
                        unsafe { self.input(c as u8) };
                    }
                    _ => {}
                }
            }
        }
    }

    /// Reads a line of input from the keyboard buffer.
    pub fn read_line(&mut self) -> Option<String> {
        let newline_index = self.line.iter().position(|&c| c == b'\n')?;
        let line = String::from_utf8_lossy(&self.line[..=newline_index]).into_owned();

        self.line[..newline_index]
            .iter_mut()
            .for_each(|f| *f = b'\0');

        // Move the remaining characters to the front of the buffer
        let remaining = self.line.len() - newline_index - 1;
        for i in 0..remaining {
            self.line[i] = self.line[newline_index + 1 + i];
        }

        self.line.truncate(remaining);

        self.new_chars = self.new_chars.saturating_sub(remaining);

        Some(line)
    }

    pub fn has_new_input(&self) -> bool {
        self.new_chars > 0
    }

    /// Reads the new input from the keyboard buffer as a string slice. If there is no new input, returns an empty string.
    pub fn read_new_input(&mut self) -> String {
        if self.new_chars == 0 {
            return String::new();
        }

        let input = &self.raw[..self.new_chars];
        let output = String::from_utf8_lossy(input).into_owned();
        self.raw.drain(..self.new_chars);
        self.new_chars = 0;
        output
    }

    pub fn backspaces(&mut self) -> usize {
        mem::replace(&mut self.backspaces, 0)
    }
}

pub(super) extern "C" fn keyboard_interrupt(frame: InterruptContext) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    let mut keyboard = unsafe { KEYBOARD.lock_interrupt() };
    unsafe {
        keyboard.scancode(scancode);
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard as u8)
    };
}

interrupt_wrapper!(keyboard_interrupt, keyboard_interrupt_raw);
