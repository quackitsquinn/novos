use core::{
    arch::asm, cmp::min, f32::consts::E, fmt::Display, num::TryFromIntError, str::FromStr,
    time::Duration,
};

use alloc::{boxed::Box, format, string::String, vec::Vec};
use cake::{
    OnceMutex,
    spin::{Mutex, MutexGuard},
};
use core::fmt::Write;
use palette::{Hsv, IntoColor, Srgb, rgb::Rgb};
use rhai::{Dynamic, Engine, EvalAltResult, ParseError, ParseErrorType, Position};

use crate::{
    display::{
        Color, ScreenChar,
        terminal::{self, Terminal},
    },
    framebuffer,
    interrupts::{
        self,
        hardware::{
            keyboard::{KEYBOARD, KeyboardDriver},
            timer::Timer,
        },
    },
    print, println, sprintln, terminal,
};

const WELCOME: &str = "
Welcome to the Novos interpreter! This is a REPL (Read-Eval-Print Loop) environment that has various built-in functions
and types you can use to interact with the system. Type 'help()' for more information.

Press ESC to clear the current input line.

";

const HELP: &str = "
This is a minimal interpreter that supports basic arithmetic and variables.
It is based off of the Rhai scripting engine, which is an embedded scripting language for Rust. 
If you want documentation for the language, visit https://rhai.rs/book/

Implemented items:

rgb(r, g, b: int) -> Color: 
    Creates a new color from the given inputs. Throws a RuntimeError
    if r, g, or b > 255 || < 0
    Color has the setters set_(r, g, b) for input validation.

hsv(h, s, v: float) -> Color:
    Creates a new color from the given inputs. Throws a RuntimeError
    if h is not in [0, 360) or s or v are not in [0, 1].

screenchar(character: char, fg, bg: Color) -> ScreenChar:
    Creates a new ScreenChar for the given character, foreground, and background.

set_char_at(x, y: int, c: ScreenChar):
    Sets a character at the given input. Will error if x/y are out of range for the terminal.

set_char_at_raw(x, y: int, c: char):
    Sets a character at the given input with white foreground and black background. Will error if x/y are out of range for the terminal.

set_cursor(x, y: int):
    Sets the cursor to the given position. Will error if x/y are out of range for the terminal.

set_foreground(c: Color):
    Sets the current foreground color for the terminal.

set_background(c: Color):
    Sets the current background color for the terminal.

set_px_at(x, y: int, c: Color):
    Sets a pixel at the given input. Will error if x/y are out of range for the framebuffer.

draw_scaled_px(x, y: int, scale: int, c: Color):
    Draws a square of size scale x scale at the given input. Will error if x/y are out of range for the framebuffer.

x_size() -> int:
    Returns the width of the terminal.

y_size() -> int:
    Returns the height of the terminal.

input(prompt: string) -> string:
    Prints the prompt and waits for user input. Returns the input string.

help():
    Prints this help message.

demo:
    A small little visual demo.

clear:
    Clears the terminal.

scope:
    Prints the current scope.
";

struct Context<'a> {
    // Option so we can take it out to mutate it
    rhai: Option<rhai::Engine>,
    scope: Option<rhai::Scope<'a>>,
    newline: bool,
    in_input: bool,
    history: Vec<String>,
    index: Option<usize>,
    lines: Vec<String>,
    unclosed_braces: usize,
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

static CONTEXT: OnceMutex<Context> = OnceMutex::uninitialized();

pub fn run() {
    interrupts::enable();
    let mut context = Context {
        rhai: Some(create_engine()),
        scope: Some(rhai::Scope::new()),
        newline: true,
        in_input: false,
        lines: Vec::new(),
        index: None,
        unclosed_braces: 0,
        history: Vec::with_capacity(100),
    };
    CONTEXT.init(context);

    {
        let mut terminal = terminal!();
        terminal.clear();
        terminal.push_str(WELCOME);
    }

    loop {
        update_display();
        update_rhai();
    }
}

fn update_display() {
    let mut context = CONTEXT.get();
    let mut driver = KEYBOARD.lock();
    if context.newline && !context.in_input {
        let mut terminal = terminal!();
        prompt(&mut terminal, &mut *context);
        context.newline = false;
    }
    context.in_input = false;
    if driver.has_new_input() {
        let mut terminal = terminal!();

        if driver.escaped() {
            context.lines.clear();
            terminal.clear_line(0);
            terminal.set_col(0);
            prompt(&mut terminal, &mut context);
            driver.set_from_history("");
        }

        update_history(&mut terminal, &mut *context, &mut *driver);
        //update_cursor(&mut terminal, &mut *context, &mut *driver);

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

fn create_invalid_error<T: Display>(fail_value: T, expected_type: &str) -> Box<EvalAltResult> {
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

fn create_engine() -> Engine {
    let mut engine = Engine::new();

    // Register functions, variables, etc. here
    engine.on_print(|s| println!("{}", s));

    engine.register_fn("help", help);

    engine.register_type::<ScreenChar>();
    engine.register_type::<Color>();

    engine.register_fn("screenchar", ScreenChar::new);
    engine.register_fn(
        "rgb",
        |r: i64, g: i64, b: i64| -> Result<Color, Box<EvalAltResult>> {
            let r = try_into_u8(r)?;
            let g = try_into_u8(g)?;
            let b = try_into_u8(b)?;
            Ok(Color::new(r, g, b))
        },
    );

    engine.register_fn("hsv", color_from_hsv);

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
    engine.register_fn("to_string", |c: &mut ScreenChar| {
        format!(
            "Screenchar('{}', fg: {}, bg: {})",
            c.character, c.foreground, c.background
        )
    });

    engine.register_fn("to_string", |c: &mut Color| {
        format!("Color(r: {}, g: {}, b: {})", c.r, c.g, c.b)
    });

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

    engine.register_fn("set_char_at", set_char_at);
    engine.register_fn("set_char_at_raw", set_char_at_raw);
    engine.register_fn("x_size", x_size);
    engine.register_fn("y_size", y_size);
    engine.register_fn("set_foreground", set_foreground);
    engine.register_fn("set_background", set_background);
    engine.register_fn("set_cursor", set_cursor);

    engine.register_fn("input", rhai_read);

    engine.register_fn("set_px_at", set_px_at);
    engine.register_fn("draw_scaled_px", draw_scaled_px);

    engine
}

fn rhai_read(base: &str) -> String {
    print!("\n{}", base);
    loop {
        {
            let mut driver = KEYBOARD.lock();
            let mut ctx = CONTEXT.get();
            if let Some(line) = driver.read_line() {
                ctx.in_input = false;
                return line;
            }
            ctx.in_input = true;
        }
        update_display();
    }
}

fn color_from_hsv(h: f64, s: f64, v: f64) -> Result<Color, Box<EvalAltResult>> {
    if !(0.0..360.0).contains(&h) {
        return Err(create_invalid_error(h as i64, "hue [0, 360)"));
    }
    if !(0.0..=1.0).contains(&s) {
        return Err(create_invalid_error(
            (s * 100.0) as i64,
            "saturation [0, 1]",
        ));
    }

    if !(0.0..=1.0).contains(&v) {
        return Err(create_invalid_error((v * 100.0) as i64, "value [0, 1]"));
    }
    let h = h as f32;
    let s = s as f32;
    let v = v as f32;
    let col: Rgb = Hsv::new(h, s, v).into_color();
    let u8_col = col.into_format::<u8>();
    Ok(Color::new(u8_col.red, u8_col.green, u8_col.blue))
}

fn update_rhai() {
    let mut driver = KEYBOARD.lock();
    let line = driver.read_line();

    if line.is_none() {
        return;
    }

    let mut context = CONTEXT.get();

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
            println!("\nType help() for help!");
            return;
        }
        "scope" => {
            println!("\nCurrent scope: {}", context.scope.as_ref().unwrap());
            return;
        }
        "clear" => {
            let mut terminal = terminal!();
            terminal.clear();
            return;
        }
        "demo" => {
            drop(driver);
            drop(context);
            demo();
            return;
        }
        _ => {}
    }

    let mut full = context.lines.join("\n");

    write!(full, "\n{}", line).expect(":(");

    let rhai = context.rhai.take().unwrap();

    let ast = rhai.compile(&full);

    context.unclosed_braces = full
        .matches('{')
        .count()
        .saturating_sub(full.matches('}').count());

    if let Err(e) = ast {
        match e.err_type() {
            ParseErrorType::UnexpectedEOF => {
                // Need more, push to lines
                context.lines.push(line);
                context.rhai = Some(rhai);
                return;
            }
            ParseErrorType::MissingToken(token, _)
                if token == "}" || token == ")" || token == "]" =>
            {
                // Need more, push to lines
                context.lines.push(line);
                context.rhai = Some(rhai);
                return;
            }

            _ => {
                println!("Compilation error! {}", e);
                context.lines.clear();
            }
        }
        context.rhai = Some(rhai);
        return;
    }

    let ast = ast.unwrap();
    // In case the ast calls `rhai_input`, we need to drop the lock on the keyboard
    drop(driver);
    let mut scope = context.scope.take().unwrap();
    drop(context);

    let res = rhai.eval_ast_with_scope::<Dynamic>(&mut scope, &ast);

    let mut context = CONTEXT.get();

    if let Err(e) = res {
        println!("\nError! {}", e);
        context.rhai = Some(rhai);
        context.scope = Some(scope);
        return;
    }

    let res = res.unwrap();
    if !res.is::<()>() {
        println!("=> {}", res)
    }
    context.lines.clear();
    context.rhai = Some(rhai);
    context.scope = Some(scope);
}

fn prompt(terminal: &mut Terminal, context: &mut Context) {
    if context.lines.len() == 0 {
        terminal.push_str(">> ");
    } else {
        // Indent slightly for multi-line input
        terminal.push_str(".. ");
        for _ in 0..context.unclosed_braces {
            terminal.push_str("  ");
        }
    }
}

fn help() {
    println!("{}", HELP);
}

fn n(val: i64, max: usize, err: &str) -> Result<usize, Box<EvalAltResult>> {
    if val < 0 {
        return Err(create_invalid_error(val, "usize"));
    }
    let val = usize::try_from(val).map_err(|_| create_invalid_error(val, "usize"))?;
    if val >= max {
        return Err(create_invalid_error(val, err));
    }
    Ok(val)
}

fn set_char_at(x: i64, y: i64, c: ScreenChar) -> Result<(), Box<EvalAltResult>> {
    let mut terminal = terminal!();

    let term_size = terminal.get_size();

    let x_usize = n(x, term_size.0, "x")?;
    let y_usize = n(y, term_size.1, "y")?;

    terminal.set_char_at(x_usize, y_usize, c);
    Ok(())
}

fn set_cursor(x: i64, y: i64) -> Result<(), Box<EvalAltResult>> {
    let mut terminal = terminal!();

    let term_size = terminal.get_size();

    let x_usize = n(x, term_size.0, "x")?;
    let y_usize = n(y, term_size.1, "y")?;

    terminal.set_cursor(x_usize, y_usize);
    Ok(())
}

fn set_foreground(c: Color) {
    let mut terminal = terminal!();
    terminal.current_fg = c;
}

fn set_background(c: Color) {
    let mut terminal = terminal!();
    terminal.current_bg = c;
}

fn set_char_at_raw(x: i64, y: i64, c: char) -> Result<(), Box<EvalAltResult>> {
    set_char_at(x, y, ScreenChar::new(c, Color::WHITE, Color::BLACK))
}

fn x_size() -> i64 {
    let terminal = terminal!();
    terminal.get_size().0 as i64
}

fn y_size() -> i64 {
    let terminal = terminal!();
    terminal.get_size().1 as i64
}

fn set_px_at(x: i64, y: i64, c: Color) -> Result<(), Box<EvalAltResult>> {
    let _terminal = terminal!();
    let mut fb = framebuffer!();
    let (x_size, y_size) = fb.size();

    let x_usize = n(x, x_size, "x")?;
    let y_usize = n(y, y_size, "y")?;

    fb.set_px(x_usize, y_usize, c);
    Ok(())
}

fn draw_scaled_px(x: i64, y: i64, scale: i64, c: Color) -> Result<(), Box<EvalAltResult>> {
    if scale <= 0 {
        return Err(create_invalid_error(scale, "scale > 0"));
    }
    let _terminal = terminal!();
    let mut fb = framebuffer!();
    let (x_size, y_size) = fb.size();

    let x_usize = n(x, x_size - (scale as usize * 2), "x")?;
    let y_usize = n(y, y_size - (scale as usize * 2), "y")?;
    let scale = n(scale, min(x_size, y_size), "scale")?;

    fb.draw_scaled_px(x_usize, y_usize, scale, c);
    Ok(())
}

// It does feel a little bit like cheating to just write the demo in Rust, but you can very much do this in Rhai too.
fn demo() {
    demo_text_chain();
    let term = terminal!();
    let mut fb = framebuffer!();
    let (x_size, y_size) = fb.size();
    for x in 0..x_size {
        for y in 0..y_size / 2 {
            let rx = (rand() % (x_size - 6));
            let ry = (rand() % (y_size - 6));
            let hue = ((rx + ry + x) % 361) as f32;
            let col: Rgb = Hsv::new(hue, 1.0, 1.0).into_color();
            let u8_col = col.into_format::<u8>();
            fb.draw_scaled_px(rx, ry, 4, Color::new(u8_col.red, u8_col.green, u8_col.blue));
        }
    }
    for x in 0..x_size {
        for y in 0..y_size / 2 {
            let rx = (rand() % (x_size - 6));
            let ry = (rand() % (y_size - 6));
            let hue = ((rx + ry + x_size + x) % 361) as f32;
            let col: Rgb = Hsv::new(hue, 1.0, 1.0 - (x as f32 / x_size as f32)).into_color();
            let u8_col = col.into_format::<u8>();
            fb.draw_scaled_px(rx, ry, 4, Color::new(u8_col.red, u8_col.green, u8_col.blue));
        }
    }
    fb.clear();
    drop(fb);
    term.force_flush();
    drop(term);
    println!(
        "Cool, right? If you want to see it again, just type 'demo'! If you want documentation, type 'help()'."
    );
}

fn timer_wait(secs: f32) {
    let timer = Timer::new(Duration::from_secs_f32(secs));
    while !timer.is_done() {
        unsafe { asm!("hlt") };
    }
}

fn demo_text_chain() {
    const WAIT: f32 = 3.0;
    println!("\nHi! What's your name?");
    let name = rhai_read("Name: ");
    println!("\nHello, {}!", name);
    let ctx = CONTEXT.get();
    timer_wait(WAIT / 2.0);
    println!("Theres all sorts of stuff you can do here.");
    timer_wait(WAIT / 2.0);
    println!("For example, you can do math:\n");
    println!(
        "2 + 2 = {}\n",
        ctx.rhai.as_ref().unwrap().eval::<i64>("2 + 2").unwrap()
    );
    timer_wait(WAIT);
    println!("You can create variables:\n");
    println!(
        "let x = 5; x * x = {}\n",
        ctx.rhai
            .as_ref()
            .unwrap()
            .eval::<i64>("let x = 5; x * x")
            .unwrap()
    );
    timer_wait(WAIT);
    println!("You can use logic:\n");
    println!(
        "if 1 < 2 {{ 3 }} else {{ 4 }} = {}\n",
        ctx.rhai
            .as_ref()
            .unwrap()
            .eval::<i64>("if 1 < 2 { 3 } else { 4 }")
            .unwrap()
    );
    timer_wait(WAIT);
    println!("You can define functions:\n");
    println!(
        "fn add(a, b) {{ a + b }}; add(3, 4) = {}\n",
        ctx.rhai
            .as_ref()
            .unwrap()
            .eval::<i64>("fn add(a, b) { a + b }; add(3, 4)")
            .unwrap()
    );
    println!(
        "You can even write to the screen directly! Tying everything together can get you something like this!\n"
    );
    timer_wait(WAIT);
    println!("Watch this...");
    timer_wait(WAIT);
    println!("Go!");
    drop(ctx);
}

fn rand() -> usize {
    // A very simple xorshift rng
    static mut STATE: u32 = 0x12345678;
    unsafe {
        STATE ^= STATE << 13;
        STATE ^= STATE >> 17;
        STATE ^= STATE << 5;
        STATE as usize
    }
}
