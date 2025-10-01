use core::{cmp::min, f32::consts::E, num::TryFromIntError, str::FromStr};

use alloc::{boxed::Box, format, string::String, vec::Vec};
use cake::spin::MutexGuard;
use core::fmt::Write;
use rhai::{Dynamic, Engine, EvalAltResult, ParseError, ParseErrorType, Position};

use crate::{
    display::{
        Color, ScreenChar,
        terminal::{self, Terminal},
    },
    interrupts::{
        self,
        hardware::keyboard::{KEYBOARD, KeyboardDriver},
    },
    print, println, sprintln, terminal,
};

const WELCOME: &str = "
Welcome to Novos, a poorly named operating system. (I'm planning to rename it soon)
Currently the kernel is in development (and will remain in development for a while) so right now it's pretty bare-bones.

Right now you are seeing a Rhai-based interpreter.
";

const HELP: &str = "
This is a minimal interpreter that supports basic arithmetic and variables.
It is based off of the Rhai scripting engine, which is an embedded scripting language for Rust. 
If you want documentation for the language, visit https://rhai.rs/book/

Implemented items:

new_color(int r, int g, int b) -> Color: 
    Creates a new color from the given inputs. Throws a RuntimeError
    if r, g, or b > 255 || < 0
    Color has the setters set_(r, g, b) for input validation.

new_screenchar(char character, Color fg, Color bg) -> ScreenChar:
    Creates a new ScreenChar for the given character, foreground, and background.

terminal() -> Terminal:
    Returns a handle to the current terminal.

Terminal::set_char_at(x: int, y: int, c: ScreenChar):
    Sets a character at the given input. Will error if x/y are out of range for the terminal.
";

struct Context<'a> {
    rhai: rhai::Engine,
    scope: rhai::Scope<'a>,
    newline: bool,
    history: Vec<String>,
    index: Option<usize>,
    lines: Vec<String>,
}

impl Context<'_> {
    fn set_index_clamped(&mut self, new_index: isize) {
        if new_index < 0 || self.history.len() == 0 {
            self.index = None;
            return;
        }

        let new_index = new_index as usize;

        if new_index >= self.history.len() {
            self.index = Some(self.history.len() - 1);
            return;
        }

        self.index = Some(new_index);
    }
}

pub fn run() {
    interrupts::enable();
    let mut context = Context {
        rhai: create_engine(),
        scope: rhai::Scope::new(),
        newline: true,
        lines: Vec::new(),
        index: None,
        history: Vec::with_capacity(100),
    };
    {
        let mut terminal = terminal!();
        terminal.clear();
        terminal.push_str(WELCOME);
    }
    loop {
        let mut keyboard = KEYBOARD.lock();
        update_display(&mut context, &mut keyboard);
        update_rhai(&mut keyboard, &mut context)
    }
}

fn update_display(context: &mut Context, driver: &mut KeyboardDriver) {
    if context.newline {
        let mut terminal = terminal!();
        prompt(&mut terminal, context);
        context.newline = false;
    }
    if driver.has_new_input() {
        let mut terminal = terminal!();

        update_history(&mut terminal, context, driver);

        for _ in 0..driver.backspaces() {
            terminal.backspace();
        }

        let new_input = driver.read_new_input();
        if new_input.contains("\n") {
            context.newline = true;
        }

        terminal.push_str(&new_input);
    }
}

fn update_history(terminal: &mut Terminal, context: &mut Context, driver: &mut KeyboardDriver) {
    let ups = driver.up_presses() as isize;
    let downs = driver.down_presses() as isize;

    if ups == 0 && downs == 0 {
        return;
    }

    let current_index = context.index.unwrap_or(context.history.len()) as isize;
    let new_index = current_index - (ups - downs);
    context.set_index_clamped(new_index);
    if let Some(index) = context.index {
        let history_line = &context.history[index];
        terminal.set_col(3);
        terminal.push_str(history_line);
        terminal.update_row();
        driver.set_from_history(history_line);
    }
}

fn create_invalid_error(fail_value: i64, expected_type: &str) -> Box<EvalAltResult> {
    return Box::new(EvalAltResult::ErrorRuntime(
        Dynamic::from_str(&format!(
            "Value out of range for {}: {}",
            expected_type, fail_value
        ))
        .expect("Infallible"),
        Position::NONE,
    ));
}

fn try_into_u8(value: i64) -> Result<u8, Box<EvalAltResult>> {
    let res = u8::try_from(value);
    if res.is_err() {
        return Err(create_invalid_error(value, "u8"));
    }
    Ok(res.unwrap())
}

fn screenchar_to_string(sc: &mut ScreenChar) -> String {
    format!(
        "Screenchar('{}', fg: {}, bg: {})",
        sc.character, sc.foreground, sc.background
    )
}

fn create_engine() -> Engine {
    let mut engine = Engine::new();

    // Register functions, variables, etc. here
    engine.on_print(|s| print!("{}", s));

    engine.register_fn("help", help);

    engine.register_type::<ScreenChar>();
    engine.register_type::<Color>();
    engine.register_type_with_name::<RhaiTerminal>("Terminal");

    engine.register_fn("screenchar_new", ScreenChar::new);
    engine.register_fn("screenchar", ScreenChar::default);
    engine.register_fn(
        "new_color",
        |r: i64, g: i64, b: i64| -> Result<Color, Box<EvalAltResult>> {
            let r = try_into_u8(r)?;
            let g = try_into_u8(g)?;
            let b = try_into_u8(b)?;
            Ok(Color::new(r, g, b))
        },
    );

    // ScreenChar::*
    engine.register_get("char", |c: &mut ScreenChar| c.character);
    engine.register_get("foreground", |c: &mut ScreenChar| c.foreground);
    engine.register_get("background", |c: &mut ScreenChar| c.background);
    engine.register_set("char", |c: &mut ScreenChar, val: char| c.character = val);
    engine.register_set("foreground", |c: &mut ScreenChar, val: Color| {
        c.foreground = val
    });
    engine.register_set("background", |c: &mut ScreenChar, val: Color| {
        c.background = val
    });
    engine.register_fn("to_string", screenchar_to_string);

    // Color::*
    engine.register_get("r", |c: &mut Color| c.r as i64);
    engine.register_get("g", |c: &mut Color| c.g as i64);
    engine.register_get("b", |c: &mut Color| c.b as i64);

    engine.register_fn(
        "set_r",
        |c: &mut Color, val: i64| -> Result<(), Box<EvalAltResult>> {
            c.r = try_into_u8(val)?;
            Ok(())
        },
    );
    engine.register_fn(
        "set_g",
        |c: &mut Color, val: i64| -> Result<(), Box<EvalAltResult>> {
            c.g = try_into_u8(val)?;
            Ok(())
        },
    );
    engine.register_fn(
        "set_b",
        |c: &mut Color, val: i64| -> Result<(), Box<EvalAltResult>> {
            c.b = try_into_u8(val)?;
            Ok(())
        },
    );

    // Terminal::*
    engine.register_fn("terminal", || RhaiTerminal);
    engine.register_fn("set_char_at", RhaiTerminal::set_char_at);
    engine.register_fn("x_size", RhaiTerminal::x_size);
    engine.register_fn("y_size", RhaiTerminal::y_size);

    engine
}

fn update_rhai(driver: &mut KeyboardDriver, context: &mut Context) {
    let line = driver.read_line();

    if line.is_none() {
        return;
    }

    let line = line.unwrap();

    let mut history_line = line.clone();
    history_line.retain(|c| c != '\n' && c != '\r');
    if history_line.len() > 0 {
        context.history.push(history_line);
    }

    context.index = None;

    sprintln!("Line: {}", line);

    match line.trim() {
        "help" => {
            println!("Type help() for help!");
            return;
        }
        "scope" => {
            println!("Current scope: {}", context.scope);
            return;
        }
        _ => {}
    }

    let mut full = context.lines.join("\n");

    write!(full, "\n{}", line).expect(":(");

    let rhai = &mut context.rhai;

    let ast = rhai.compile(&full);

    if let Err(e) = ast {
        match e.err_type() {
            ParseErrorType::UnexpectedEOF => {
                // Need more, push to lines
                context.lines.push(line);
                return;
            }
            ParseErrorType::MissingToken(token, _)
                if token == "}" || token == ")" || token == "]" =>
            {
                // Need more, push to lines
                context.lines.push(line);
                return;
            }

            _ => {
                println!("Compilation error! {}", e);
            }
        }
        return;
    }

    let ast = ast.unwrap();

    let res = rhai.eval_ast_with_scope::<Dynamic>(&mut context.scope, &ast);

    if let Err(e) = res {
        println!("Error! {}", e);
        return;
    }

    let res = res.unwrap();
    if !res.is::<()>() {
        println!("=> {}", res)
    }
}

fn prompt(terminal: &mut Terminal, context: &mut Context) {
    terminal.push_str(">> ");
}

fn help() {
    println!("{}", HELP);
}

#[derive(Debug, Clone)]
struct RhaiTerminal;

impl RhaiTerminal {
    fn _lock_term() -> MutexGuard<'static, Terminal> {
        terminal!()
    }

    fn set_char_at(&mut self, x: i64, y: i64, c: ScreenChar) -> Result<(), Box<EvalAltResult>> {
        let mut terminal = Self::_lock_term();

        let x_usize = usize::try_from(x).map_err(|_| create_invalid_error(x, "usize"))?;
        let y_usize = usize::try_from(y).map_err(|_| create_invalid_error(y, "usize"))?;

        let term_size = terminal.get_size();

        if x_usize >= term_size.0 {
            return Err(create_invalid_error(x, "terminal x"));
        }

        if y_usize >= term_size.1 {
            return Err(create_invalid_error(x, "terminal y"));
        }

        terminal.set_char_at(x_usize, y_usize, c);
        Ok(())
    }

    fn x_size(&mut self) -> i64 {
        let mut terminal = Self::_lock_term();
        terminal.get_size().0 as i64
    }

    fn y_size(&mut self) -> i64 {
        let mut terminal = Self::_lock_term();
        terminal.get_size().1 as i64
    }
}
