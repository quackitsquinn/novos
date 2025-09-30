use core::f32::consts::E;

use alloc::{string::String, vec::Vec};
use cake::spin::MutexGuard;
use core::fmt::Write;
use rhai::{Dynamic, Engine, ParseError, ParseErrorType};

use crate::{
    display::{
        terminal::{self, Terminal},
        Color, ScreenChar,
    },
    interrupts::{
        self,
        hardware::keyboard::{KeyboardDriver, KEYBOARD},
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
";

struct Context<'a> {
    rhai: rhai::Engine,
    scope: rhai::Scope<'a>,
    newline: bool,
    lines: Vec<String>,
}

pub fn run() {
    interrupts::enable();
    let mut context = Context {
        rhai: create_engine(),
        scope: rhai::Scope::new(),
        newline: true,
        lines: Vec::new(),
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

fn create_engine() -> Engine {
    let mut engine = Engine::new();

    // Register functions, variables, etc. here
    engine.on_print(|s| print!("{}", s));

    engine.register_fn("help", help);

    engine.register_type::<ScreenChar>();
    engine.register_type::<Color>();
    engine.register_type_with_name::<RhaiTerminal>("Terminal");

    engine.register_fn("new_screenchar", ScreenChar::new);
    engine.register_fn("new_color", Color::new);

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

    // Color::*
    engine.register_get_set("r", |c: &mut Color| c.r, |c: &mut Color, val: u8| c.r = val);
    engine.register_get_set("g", |c: &mut Color| c.g, |c: &mut Color, val: u8| c.g = val);
    engine.register_get_set("b", |c: &mut Color| c.b, |c: &mut Color, val: u8| c.b = val);

    // Terminal::*
    engine.register_fn("set_char_at", RhaiTerminal::set_char_at);

    engine
}

fn update_rhai(driver: &mut KeyboardDriver, context: &mut Context) {
    let line = driver.read_line();

    if line.is_none() {
        return;
    }

    let line = line.unwrap();

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
        "functions" => {
            for (name, is_const, item) in context.scope.iter() {
                if item.is_fnptr() {
                    println!("Function: {}", name);
                }
            }

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
            ParseErrorType::MissingToken(token, _) if token == "}" => {
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

    fn set_char_at(x: usize, y: usize, c: ScreenChar) {
        let mut terminal = Self::_lock_term();
        terminal.set_char_at(x, y, c);
    }
}
